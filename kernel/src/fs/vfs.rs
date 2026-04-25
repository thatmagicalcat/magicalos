use alloc::{collections::btree_map::BTreeMap, string::String, sync::Arc, vec::Vec};
use slotmap::{SlotMap, new_key_type};

use crate::{
    dbg_print, dbg_println,
    fs::{
        OpenOptions,
        data_handle::{DataHandle, DynamicData, StaticData},
    },
    io::{self, IoInterface},
};

new_key_type! { pub struct VfsNodeId; }

pub enum VfsNode {
    File(VfsFile),
    Directory(VfsDirectory),
}

impl VfsNode {
    pub const fn is_dir(&self) -> bool {
        matches!(self, Self::Directory(..))
    }

    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(..))
    }

    pub const fn as_file(&self) -> io::Result<&VfsFile> {
        match self {
            Self::File(f) => Ok(f),
            _ => Err(io::Error::NotAFile),
        }
    }

    pub const fn as_file_mut(&mut self) -> io::Result<&mut VfsFile> {
        match self {
            Self::File(f) => Ok(f),
            _ => Err(io::Error::NotAFile),
        }
    }

    pub const fn as_dir(&self) -> io::Result<&VfsDirectory> {
        match self {
            Self::Directory(d) => Ok(d),
            _ => Err(io::Error::NotADirectory),
        }
    }

    pub const fn as_dir_mut(&mut self) -> io::Result<&mut VfsDirectory> {
        match self {
            Self::Directory(d) => Ok(d),
            _ => Err(io::Error::NotADirectory),
        }
    }
}

#[derive(Debug)]
pub struct VfsFile {
    data: DataHandle,
}

impl VfsFile {
    pub fn new_dynamic(writable: bool) -> Self {
        Self {
            data: DataHandle::Dynamic(DynamicData::new(writable)),
        }
    }

    pub fn new_static(data: &'static [u8]) -> Self {
        Self {
            data: DataHandle::Static(StaticData::new(data)),
        }
    }

    fn get_handle(&self, flags: OpenOptions) -> io::Result<Arc<dyn IoInterface>> {
        Ok(Arc::new(Self {
            data: self.data.get_handle(flags),
        }))
    }
}

impl IoInterface for VfsFile {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        match &self.data {
            DataHandle::Dynamic(dynamic_data) => dynamic_data.read(buf),
            DataHandle::Static(static_data) => static_data.read(buf),
        }
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        match &self.data {
            DataHandle::Dynamic(dynamic_data) => dynamic_data.write(buf),
            _ => Err(io::Error::BadFileDescriptor),
        }
    }

    fn seek(&self, offset: super::SeekFrom) -> io::Result<usize> {
        self.data.seek(offset)
    }
}

impl From<VfsFile> for VfsNode {
    fn from(value: VfsFile) -> Self {
        Self::File(value)
    }
}

pub struct VfsDirectory {
    parent: Option<VfsNodeId>,
    children: BTreeMap<String, VfsNodeId>,
}

impl From<VfsDirectory> for VfsNode {
    fn from(value: VfsDirectory) -> Self {
        Self::Directory(value)
    }
}

pub struct Vfs {
    arena: SlotMap<VfsNodeId, VfsNode>,
    root: VfsNodeId,
}

impl Vfs {
    pub fn new() -> Self {
        let mut arena: SlotMap<VfsNodeId, VfsNode> = SlotMap::with_key();

        let root = arena.insert(VfsNode::Directory(VfsDirectory {
            parent: None,
            children: BTreeMap::new(),
        }));

        Self { arena, root }
    }

    pub const fn get_root_node_id(&self) -> VfsNodeId {
        self.root
    }

    pub fn get_root_node(&self) -> &VfsNode {
        &self.arena[self.root]
    }

    pub fn get_root_dir(&self) -> &VfsDirectory {
        match self.get_root_node() {
            VfsNode::Directory(vfs_directory) => vfs_directory,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_node(&self, id: VfsNodeId) -> io::Result<&VfsNode> {
        self.arena.get(id).ok_or(io::Error::NoSuchFileOrDirectory)
    }

    #[inline]
    pub fn get_node_mut(&mut self, id: VfsNodeId) -> io::Result<&mut VfsNode> {
        self.arena
            .get_mut(id)
            .ok_or(io::Error::NoSuchFileOrDirectory)
    }

    pub fn resolve_path(&self, cwd: VfsNodeId, path: &str) -> io::Result<VfsNodeId> {
        assert!(matches!(self.get_node(cwd), Ok(VfsNode::Directory(..))));

        let mut current_id = if path.starts_with('/') {
            self.root
        } else {
            cwd
        };

        for component in path.split('/') {
            match component {
                "" | "." => continue,
                ".." => {
                    let node = self.arena.get(current_id).ok_or(io::Error::StaleId)?;
                    if let VfsNode::Directory(VfsDirectory { parent, .. }) = node {
                        if let Some(parent_id) = parent {
                            current_id = *parent_id;
                        }
                    } else {
                        // the assertion above should take care of this
                        unreachable!()
                    }
                }

                child_name => {
                    let node = self.arena.get(current_id).ok_or(io::Error::StaleId)?;
                    if let VfsNode::Directory(VfsDirectory { children, .. }) = node {
                        if let Some(&child_id) = children.get(child_name) {
                            current_id = child_id;
                        } else {
                            return Err(io::Error::NoSuchFileOrDirectory);
                        }
                    } else {
                        unreachable!()
                    }
                }
            }
        }

        Ok(current_id)
    }

    pub fn mkdir(&mut self, cwd: VfsNodeId, path: &str) -> io::Result<VfsNodeId> {
        if path == "/" {
            return Err(io::Error::AlreadyExists);
        }

        // /usr/bin/ -> /usr/bin
        let path = path.trim_end_matches('/');

        let (new_dir_name, parent_id) = self.split_leaf(cwd, path)?;

        // verify that the child doesn't already exists
        let parent_node = self.get_node(parent_id)?;
        match parent_node {
            VfsNode::Directory(vfs_directory)
                if vfs_directory.children.contains_key(new_dir_name) =>
            {
                return Err(io::Error::AlreadyExists);
            }

            VfsNode::Directory(..) => {}

            _ => return Err(io::Error::NotADirectory),
        }

        let new_dir_id = self.arena.insert(
            VfsDirectory {
                parent: Some(parent_id),
                children: BTreeMap::new(),
            }
            .into(),
        );

        if let VfsNode::Directory(VfsDirectory { children, .. }) = self.get_node_mut(parent_id)? {
            children.insert(String::from(new_dir_name), new_dir_id);
        }

        Ok(new_dir_id)
    }

    pub fn mount(
        &mut self,
        cwd: VfsNodeId,
        path: &str,
        region: &'static [u8],
    ) -> io::Result<VfsNodeId> {
        let (file_name, parent_id) = self.split_leaf(cwd, path)?;

        let parent_node = self.get_node(parent_id)?.as_dir()?;
        if parent_node.children.contains_key(file_name) {
            return Err(io::Error::AlreadyExists);
        }

        let file_node: VfsNode = VfsFile::new_static(region).into();
        let file_id = self.arena.insert(file_node);
        let parent_node = self.get_node_mut(parent_id)?.as_dir_mut()?;

        parent_node
            .children
            .insert(String::from(file_name), file_id);
        Ok(file_id)
    }

    pub fn open(
        &mut self,
        cwd: VfsNodeId,
        path: &str,
        flags: OpenOptions,
    ) -> io::Result<Arc<dyn IoInterface>> {
        // it doesn't, we have to create it
        let (file_name, parent_id) = self.split_leaf(cwd, path)?;

        if let Some(handle) = self
            .get_node(parent_id)?
            .as_dir()?
            .children
            .get(file_name)
            .and_then(|&id| self.get_node(id).ok())
            .and_then(|node| node.as_file().ok())
            .and_then(|file| file.get_handle(flags).ok())
        {
            return Ok(handle);
        }

        if flags.contains(OpenOptions::CREATE) {
            let file = VfsFile::new_dynamic(true);
            let handle = file.get_handle(flags);
            let file_id = self.arena.insert(file.into());
            let parent_dir = self.get_node_mut(parent_id)?.as_dir_mut()?;

            parent_dir.children.insert(String::from(file_name), file_id);
            return handle;
        }

        Err(io::Error::InvalidValue)
    }

    fn split_leaf<'a>(
        &mut self,
        cwd: VfsNodeId,
        path: &'a str,
    ) -> Result<(&'a str, VfsNodeId), io::Error> {
        let (parent, leaf) = match path.rsplit_once('/') {
            Some((parent, name)) => {
                // /home -> ("", "home") -> ("/", "home")
                let resolved_parent = if parent.is_empty() { "/" } else { parent };
                (resolved_parent, name)
            }
            None => ("", path), // parent is cwd
        };

        if matches!(leaf, "" | "." | "..") {
            return Err(io::Error::InvalidValue);
        }

        let parent_id = if parent.is_empty() {
            cwd
        } else {
            self.resolve_path(cwd, parent)?
        };

        Ok((leaf, parent_id))
    }

    pub fn tree_lsdir(&self, dir_id: VfsNodeId) {
        let dir_node = self.get_node(dir_id);
        assert!(
            matches!(dir_node, Ok(VfsNode::Directory(..))),
            "Not a directory"
        );

        fn helper(
            this: &VfsDirectory,
            prefixes: &mut Vec<bool>,
            arena: &SlotMap<VfsNodeId, VfsNode>,
        ) {
            let children_count = this.children.len();

            for (i, (node_name, node_id)) in this.children.iter().enumerate() {
                let is_last = i == children_count - 1;

                for &parent_is_last in prefixes.iter() {
                    if parent_is_last {
                        dbg_print!("    ");
                    } else {
                        dbg_print!("│   ");
                    }
                }

                if is_last {
                    dbg_print!("└── ");
                } else {
                    dbg_print!("├── ");
                }

                if let VfsNode::Directory(vfs_directory) = &arena[*node_id] {
                    dbg_println!(" {node_name}");

                    prefixes.push(is_last);
                    helper(vfs_directory, prefixes, arena);
                    prefixes.pop();
                } else {
                    dbg_println!(" {node_name}");
                }
            }
        }

        helper(
            dir_node.unwrap().as_dir().unwrap(),
            &mut Vec::new(),
            &self.arena,
        );
    }
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

#[test_case]
fn vfs_rewrite_test() {
    let mut vfs = Vfs::new();
}
