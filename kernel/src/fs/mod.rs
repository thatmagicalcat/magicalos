use core::{any::Any, fmt::Debug};
use alloc::{sync::Arc, vec::Vec};
use enum_dispatch::enum_dispatch;

use vfs::{VfsDirectory, VfsFile};
use crate::io::{self, IoInterface};

mod data_handle;
mod error;
mod vfs;
pub mod tar;

static mut VFS_ROOT: Option<vfs::VfsRoot> = None;

pub fn init_vfs() {
    log::info!("Initializing VFS...");

    let root = vfs::VfsRoot::new();

    root.mkdir("/home/thatmagicalcat/").unwrap();
    root.mkdir("/bin").unwrap();
    root.mkdir("/dev").unwrap();

    static MESSAGE: &[u8] = b"Hello, World!\n";

    root.mount("/home/thatmagicalcat/message.txt", MESSAGE).unwrap();

    root.lsdir().unwrap();

    unsafe {
        VFS_ROOT = Some(root);
    }
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
