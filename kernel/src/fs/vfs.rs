use core::any::Any;

use crate::{
    dbg_print, dbg_println,
    fs::data_handle::{DynamicData, StaticData},
    io::{self, IoInterface},
    synch::Spinlock,
};

use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};

use super::{
    NodeKind, OpenOptions, VfsDirectoryNode, VfsFileNode, VfsNode, VfsNodeEnum,
    data_handle::DataHandle,
};

pub(crate) struct VfsRoot {
    inner: Spinlock<VfsDirectory>,
}

impl VfsRoot {
    pub const fn new() -> Self {
        Self {
            inner: Spinlock::new(VfsDirectory::new()),
        }
    }

    pub fn mkdir(&self, path: &str) -> io::Result<()> {
        if !check_path(path) {
            return Err(io::Error::NoSuchFileOrDirectory);
        }

        let mut components: Vec<&str> = path.trim_end_matches('/').split('/').collect();

        // example: /home/user/file.txt
        //
        // split   -> ["", "home", "user", "file.txt"]
        // reverse -> ["file.txt", "user", "home", ""]
        // pop     -> ["file.txt", "user", "home"]

        components.reverse();
        components.pop(); // pop the empty component before the first '/'

        self.inner.lock().mkdir(&mut components);
        Ok(())
    }

    pub fn open(&self, path: &str, flags: OpenOptions) -> io::Result<Arc<dyn IoInterface>> {
        if !check_path(path) {
            return Err(io::Error::NoSuchFileOrDirectory);
        }

        let mut components: Vec<&str> = path.trim_end_matches('/').split('/').collect();
        components.reverse();
        components.pop();

        self.inner.lock().open(&mut components, flags)
    }

    pub fn mount(&self, path: &str, region: &'static [u8]) -> io::Result<()> {
        if !check_path(path) {
            return Err(io::Error::NoSuchFileOrDirectory);
        }

        let mut components: Vec<&str> = path.trim_end_matches('/').split('/').collect();
        components.reverse();
        components.pop();

        self.inner.lock().mount(&mut components, region)
    }

    pub fn lsdir(&self) -> io::Result<()> {
        self.inner.lock().tree_lsdir();
        Ok(())
    }
}

fn check_path(path: &str) -> bool {
    path.starts_with('/')
}

#[derive(Debug)]
pub(crate) struct VfsDirectory {
    children: BTreeMap<String, VfsNodeEnum>,
}

impl VfsDirectory {
    pub const fn new() -> Self {
        Self {
            children: BTreeMap::new(),
        }
    }

    fn get<T: Any>(&self, name: &str) -> Option<&T> {
        self.children.get(name)?.as_any().downcast_ref::<T>()
    }

    fn get_mut<T: Any>(&mut self, name: &str) -> Option<&mut T> {
        self.children
            .get_mut(name)?
            .as_any_mut()
            .downcast_mut::<T>()
    }
}

impl VfsDirectoryNode for VfsDirectory {
    fn mkdir(&mut self, components: &mut Vec<&str>) {
        let Some(node_name) = components.pop() else {
            // reached the end
            return;
        };

        if let Some(dir) = self.get_mut::<Self>(node_name) {
            dir.mkdir(components);
        } else {
            let mut dir = VfsDirectory::new();
            dir.mkdir(components);
            self.children.insert(String::from(node_name), dir.into());
        }
    }

    fn tree_lsdir(&self) {
        dbg_println!("/");
        fn helper(this: &VfsDirectory, prefixes: &mut Vec<bool>) {
            let children_count = this.children.len();

            for (i, (node_name, node)) in this.children.iter().enumerate() {
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

                if let Some(dir_node) = node.as_any().downcast_ref::<VfsDirectory>() {
                    dbg_println!(" {node_name}");

                    prefixes.push(is_last);
                    helper(dir_node, prefixes);
                    prefixes.pop();
                } else {
                    dbg_println!(" {node_name}");
                }
            }
        }

        helper(self, &mut Vec::new());
    }

    fn open(
        &mut self,
        components: &mut Vec<&str>,
        flags: OpenOptions,
    ) -> io::Result<Arc<dyn IoInterface>> {
        let Some(node_name) = components.pop() else {
            return Err(io::Error::InvalidValue);
        };

        // if this was the last component, then it must be the file name
        if components.is_empty() {
            // if the file exists, open it
            if let Some(file_node) = self.get_mut::<VfsFile>(node_name) {
                return file_node.get_handle(flags);
            };

            // it doesn't exists, we can create it if the options allow it
            if flags.contains(OpenOptions::CREATE) {
                let file = VfsFile::new_dynamic(true);
                let handle = file.get_handle(flags);
                self.children.insert(String::from(node_name), file.into());

                return handle;
            }

            return Err(io::Error::InvalidValue);
        }

        // current node must be a directory
        let Some(dir_node) = self.get_mut::<VfsDirectory>(node_name) else {
            return Err(io::Error::NotADirectory);
        };

        dir_node.open(components, flags)
    }

    fn mount(&mut self, components: &mut Vec<&str>, region: &'static [u8]) -> io::Result<()> {
        let Some(node_name) = components.pop() else {
            return Err(io::Error::InvalidValue);
        };

        // if this was the last component, then it must be the file name
        if components.is_empty() {
            let file = VfsFile::new_static(region);
            self.children.insert(String::from(node_name), file.into());
            return Ok(());
        }

        // current node must be a directory
        let Some(dir_node) = self.get_mut::<VfsDirectory>(node_name) else {
            return Err(io::Error::NotADirectory);
        };

        dir_node.mount(components, region)
    }
}

impl VfsNode for VfsDirectory {
    fn get_node_kind(&self) -> NodeKind {
        NodeKind::Directory
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
pub(crate) struct VfsFile {
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
}

impl VfsNode for VfsFile {
    fn get_node_kind(&self) -> NodeKind {
        NodeKind::File
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl VfsFileNode for VfsFile {
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
