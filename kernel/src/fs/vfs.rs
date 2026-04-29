use alloc::{collections::btree_map::BTreeMap, string::String, sync::Arc, vec::Vec};
use slotmap::{SlotMap, new_key_type};

use crate::{
    dbg_print, dbg_println,
    fs::{
        OpenOptions,
        data_handle::{DataHandle, DynamicData, StaticData},
    },
    io::{self, IoInterface}
};

new_key_type! { pub struct VfsNodeId; }

pub enum VfsNode {
    File(VfsFile),
    Directory(VfsDirectory),
    Device(Arc<dyn IoInterface>),
}

#[allow(dead_code)]
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
    children: spin::RwLock<BTreeMap<String, VfsNodeId>>,
}

impl From<VfsDirectory> for VfsNode {
    fn from(value: VfsDirectory) -> Self {
        Self::Directory(value)
    }
}

pub struct Vfs {
    arena: spin::RwLock<SlotMap<VfsNodeId, VfsNode>>,
    root: VfsNodeId,
}

impl Vfs {
    pub fn new() -> Self {
        let mut arena: SlotMap<VfsNodeId, VfsNode> = SlotMap::with_key();

        let root = arena.insert(VfsNode::Directory(VfsDirectory {
            parent: None,
            children: spin::RwLock::new(BTreeMap::new()),
        }));

        Self {
            arena: spin::RwLock::new(arena),
            root,
        }
    }

    pub const fn get_root_node_id(&self) -> VfsNodeId {
        self.root
    }

    pub fn resolve_path(&self, cwd: VfsNodeId, path: &str) -> io::Result<VfsNodeId> {
        if !matches!(self.arena.read().get(cwd), Some(VfsNode::Directory(..))) {
            return Err(io::Error::NotADirectory);
        }

        let mut current_id = if path.starts_with('/') {
            self.root
        } else {
            cwd
        };

        for component in path.split('/') {
            match component {
                "" | "." => continue,
                ".." => {
                    let read = self.arena.read();
                    let node = read.get(current_id).ok_or(io::Error::StaleId)?;
                    if let VfsNode::Directory(VfsDirectory { parent, .. }) = node {
                        if let Some(parent_id) = parent {
                            current_id = *parent_id;
                        }
                    } else {
                        return Err(io::Error::NotADirectory);
                    }
                }

                child_name => {
                    let read = self.arena.read();
                    let node = read.get(current_id).ok_or(io::Error::StaleId)?;
                    if let VfsNode::Directory(VfsDirectory { children, .. }) = node {
                        if let Some(&child_id) = children.read().get(child_name) {
                            current_id = child_id;
                        } else {
                            return Err(io::Error::NoSuchFileOrDirectory);
                        }
                    } else {
                        return Err(io::Error::NotADirectory);
                    }
                }
            }
        }

        Ok(current_id)
    }

    pub fn mkdir(&self, cwd: VfsNodeId, path: &str) -> io::Result<VfsNodeId> {
        if path == "/" {
            return Err(io::Error::AlreadyExists);
        }

        // /usr/bin/ -> /usr/bin
        let path = path.trim_end_matches('/');
        let (new_dir_name, parent_id) = self.split_leaf(cwd, path)?;

        // optimistic-allocation
        let new_dir_id = self.arena.write().insert(
            VfsDirectory {
                parent: Some(parent_id),
                children: spin::RwLock::new(BTreeMap::new()),
            }
            .into(),
        );

        // verify that the child doesn't already exists
        let arena_r = self.arena.read();
        let Some(parent_node) = arena_r.get(parent_id) else {
            drop(arena_r);
            self.arena.write().remove(new_dir_id); // rollback
            return Err(io::Error::NoSuchFileOrDirectory);
        };

        let parent_dir = parent_node.as_dir()?;
        let mut children_w = parent_dir.children.write();

        if children_w.contains_key(new_dir_name) {
            drop(children_w);
            drop(arena_r);

            // rollback optimistic-allocation
            self.arena.write().remove(new_dir_id);
            return Err(io::Error::AlreadyExists);
        }

        children_w.insert(String::from(new_dir_name), new_dir_id);

        Ok(new_dir_id)
    }

    pub fn mount(
        &self,
        cwd: VfsNodeId,
        path: &str,
        region: &'static [u8],
    ) -> io::Result<VfsNodeId> {
        let (file_name, parent_id) = self.split_leaf(cwd, path)?;

        // optimistic-allocation
        let new_file_node: VfsNode = VfsFile::new_static(region).into();
        let new_file_id = self.arena.write().insert(new_file_node);

        let arena_r = self.arena.read();
        let Some(parent_node) = arena_r.get(parent_id) else {
            drop(arena_r);
            self.arena.write().remove(new_file_id); // rollback
            return Err(io::Error::NoSuchFileOrDirectory);
        };

        let parent_dir = parent_node.as_dir()?;
        let mut children_w = parent_dir.children.write();

        if children_w.contains_key(file_name) {
            drop(children_w);
            drop(arena_r);
            self.arena.write().remove(new_file_id); // Rollback!

            return Err(io::Error::AlreadyExists);
        }

        children_w.insert(String::from(file_name), new_file_id);
        Ok(new_file_id)
    }

    pub fn open(
        &self,
        cwd: VfsNodeId,
        path: &str,
        flags: OpenOptions,
    ) -> io::Result<Arc<dyn IoInterface>> {
        let (file_name, parent_id) = self.split_leaf(cwd, path)?;

        // fast check:
        // return the handle if it already exists
        let arena_r = self.arena.read();
        let parent_dir = arena_r
            .get(parent_id)
            .ok_or(io::Error::NoSuchFileOrDirectory)?
            .as_dir()?;

        if let Some(&file_id) = parent_dir.children.read().get(file_name) {
            let node = arena_r.get(file_id).ok_or(io::Error::StaleId)?;

            return match node {
                VfsNode::File(vfs_file) => vfs_file.get_handle(flags),
                VfsNode::Device(handle) => Ok(Arc::clone(handle)),

                _ => return Err(io::Error::NotAFile),
            };
        }

        drop(arena_r); // for optimistic-allocation

        // if it doesn't exist and we aren't creating
        // fail
        if !flags.contains(OpenOptions::CREAT) {
            return Err(io::Error::NoSuchFileOrDirectory);
        }

        // optimistic-allocation
        let file = VfsFile::new_dynamic(true);
        let handle = file.get_handle(flags)?;
        let new_file_id = self.arena.write().insert(file.into());

        // acquire the lock to commit
        let arena_r = self.arena.read();
        let Some(parent_node) = arena_r.get(parent_id) else {
            drop(arena_r);
            self.arena.write().remove(new_file_id); // rollback
            return Err(io::Error::NoSuchFileOrDirectory);
        };

        let parent_dir = parent_node.as_dir()?;
        let mut children_w = parent_dir.children.write();

        // check once again, did someone else create it while we dropped the lock?
        if let Some(&existing_file_id) = children_w.get(file_name) {
            drop(children_w);

            let existing_handle = match arena_r.get(existing_file_id).ok_or(io::Error::StaleId)? {
                VfsNode::File(vfs_file) => vfs_file.get_handle(flags),
                VfsNode::Device(io_interface) => Ok(Arc::clone(io_interface)),

                _ => return Err(io::Error::NotAFile),
            };

            // rollback optimistic-allocation
            drop(arena_r);
            self.arena.write().remove(new_file_id);

            return existing_handle;
        }

        // commit
        children_w.insert(String::from(file_name), new_file_id);
        Ok(handle)
    }

    // TODO: add deferred deletion
    pub fn unlink(&self, cwd: VfsNodeId, path: &str) -> io::Result<()> {
        let (file_name, parent_id) = self.split_leaf(cwd, path)?;

        let target_id = {
            let arena_r = self.arena.read();
            let parent_dir = arena_r
                .get(parent_id)
                .ok_or(io::Error::NoSuchFileOrDirectory)?
                .as_dir()?;
            let mut children_w = parent_dir.children.write();

            let Some(&target_id) = children_w.get(file_name) else {
                return Err(io::Error::NoSuchFileOrDirectory);
            };

            let target_node = arena_r.get(target_id).ok_or(io::Error::StaleId)?;
            if !target_node.is_file() {
                return Err(io::Error::NotAFile);
            }

            children_w.remove(file_name);
            target_id
        };

        self.arena.write().remove(target_id);

        Ok(())
    }

    pub fn rmdir(&self, cwd: VfsNodeId, path: &str) -> io::Result<()> {
        let (file_name, parent_id) = self.split_leaf(cwd, path)?;

        let target_id = {
            let arena_r = self.arena.read();
            let parent_dir = arena_r
                .get(parent_id)
                .ok_or(io::Error::NoSuchFileOrDirectory)?
                .as_dir()?;
            let mut children_w = parent_dir.children.write();

            let Some(&target_id) = children_w.get(file_name) else {
                return Err(io::Error::NoSuchFileOrDirectory);
            };

            if target_id == self.root {
                return Err(io::Error::InvalidValue);
            }

            let target_node = arena_r.get(target_id).ok_or(io::Error::StaleId)?;
            if !target_node.as_dir()?.children.read().is_empty() {
                return Err(io::Error::DirectoryNotEmpty);
            }

            children_w.remove(file_name);
            target_id
        };

        self.arena.write().remove(target_id);

        Ok(())
    }

    fn split_leaf<'a>(
        &self,
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

    pub fn tree_lsdir(&self, dir_id: VfsNodeId) -> io::Result<()> {
        fn helper(
            this: &VfsDirectory,
            prefixes: &mut Vec<bool>,
            arena: &spin::RwLockReadGuard<SlotMap<VfsNodeId, VfsNode>>,
        ) {
            let children = this.children.read();
            let children_count = children.len();

            for (i, (node_name, node_id)) in children.iter().enumerate() {
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

                let vfs_node = &arena[*node_id];
                if let VfsNode::Directory(vfs_directory) = vfs_node {
                    dbg_println!(" {node_name}");

                    prefixes.push(is_last);
                    helper(vfs_directory, prefixes, arena);
                    prefixes.pop();
                } else {
                    match vfs_node {
                        VfsNode::File(_) => dbg_println!(" {node_name}"),
                        VfsNode::Device(_) => dbg_println!(" {node_name}"),

                        VfsNode::Directory(_) => unreachable!(),
                    }
                }
            }
        }

        let arena_r = self.arena.read();
        helper(
            arena_r
                .get(dir_id)
                .ok_or(io::Error::NoSuchFileOrDirectory)?
                .as_dir()?,
            &mut Vec::new(),
            &arena_r,
        );

        Ok(())
    }

    pub fn register_device(
        &self,
        cwd: VfsNodeId,
        path: &str,
        handle: Arc<dyn IoInterface>,
    ) -> io::Result<()> {
        let (file_name, parent_id) = self.split_leaf(cwd, path)?;

        let device_node = VfsNode::Device(handle);
        let device_node_id = self.arena.write().insert(device_node);

        let arena_r = self.arena.read();
        let Some(parent_node) = arena_r.get(parent_id) else {
            drop(arena_r);
            self.arena.write().remove(device_node_id); // rollback
            return Err(io::Error::NoSuchFileOrDirectory);
        };

        let parent_dir = parent_node.as_dir()?;
        let mut children_w = parent_dir.children.write();

        if children_w.contains_key(file_name) {
            drop(children_w);
            drop(arena_r);
            self.arena.write().remove(device_node_id); // rollback

            return Err(io::Error::AlreadyExists);
        }

        children_w.insert(String::from(file_name), device_node_id);
        Ok(())
    }
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}
