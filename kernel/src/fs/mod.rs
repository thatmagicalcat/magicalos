use alloc::{sync::Arc, vec::Vec};
use core::{any::Any, fmt::Debug};
use enum_dispatch::enum_dispatch;

use crate::{
    fs::tar::{TarEntiresIterator, TarEntry},
    io::{self, IoInterface},
    limine_requests,
};
use vfs::{VfsDirectory, VfsFile};

mod data_handle;
mod error;
pub mod tar;
mod vfs;

static mut VFS_ROOT: Option<vfs::VfsRoot> = None;

fn load_ramfs() -> TarEntiresIterator<'static> {
    log::info!("Loading ramfs module");

    let module = unsafe { *limine_requests::MODULE_REQUEST.response };
    let len = module.module_count as usize;
    let modules = unsafe { core::slice::from_raw_parts(module.modules, len) };
    let ramfs_module_file = *modules
        .iter()
        .find(|&&m| {
            let path = unsafe { core::ffi::CStr::from_ptr((*m).string) };
            path.to_str() == Ok("ramfs")
        })
        .expect("Failed to find ramfs module");

    let ramfs_module_raw: &'static [u8] = unsafe {
        core::slice::from_raw_parts(
            (*ramfs_module_file).address as *const u8,
            (*ramfs_module_file).size as usize,
        )
    };

    TarEntiresIterator::new(ramfs_module_raw)
}

pub fn init_vfs() {
    log::info!("Initializing VFS...");

    let root = vfs::VfsRoot::new();
    let ramfs = load_ramfs();

    for entry in ramfs {
        match entry {
            TarEntry::File { name, data } => {
                log::debug!(
                    "init_vfs(): mounting {} (size: {} bytes)",
                    &name[1..],
                    data.len()
                );
                root.mount(&name[1..], data).expect("failed to mount");
            }

            TarEntry::Directory { name } => {
                log::debug!("init_vfs(): creating directory {}", &name[1..]);
                root.mkdir(&name[1..]).expect("failed to create directory");
            }

            TarEntry::Other { .. } => log::error!("Other tar entry types are not supported yet :("),
        }
    }

    log::info!("Kernel VFS Tree:");
    root.lsdir().unwrap();

    unsafe { VFS_ROOT = Some(root) };
}

pub enum SeekFrom {
    /// Set the position to the specified number of bytes.
    Start(usize),
    /// Set the position to the current position plus the specified number of bytes.
    Current(isize),
    /// Set the position to the end of the stream plus the specified number of bytes.
    End(isize),
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenOptions: u32 {
        const RDONLY = 1 << 0;
        const WRONLY = 1 << 1;
        const APPEND = 1 << 2;
        const TRUNCATE = 1 << 3;
        const CREATE = 1 << 4;

        const RW = Self::RDONLY.bits() | Self::WRONLY.bits();
    }
}

pub(crate) enum NodeKind {
    File,
    Directory,
}

#[enum_dispatch]
#[derive(Debug)]
pub(crate) enum VfsNodeEnum {
    VfsDirectory,
    VfsFile,
}

#[enum_dispatch(VfsNodeEnum)]
trait VfsNode: Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn get_node_kind(&self) -> NodeKind;
}

trait VfsDirectoryNode {
    fn mkdir(&mut self, components: &mut Vec<&str>);
    fn mount(&mut self, components: &mut Vec<&str>, region: &'static [u8]) -> io::Result<()>;
    fn open(
        &mut self,
        components: &mut Vec<&str>,
        options: OpenOptions,
    ) -> io::Result<Arc<dyn IoInterface>>;
    /// Recursively print all the nodes inside this directory in a tree format
    fn tree_lsdir(&self);
}

trait VfsFileNode {
    fn get_handle(&self, flags: OpenOptions) -> io::Result<Arc<dyn IoInterface>>;
}
