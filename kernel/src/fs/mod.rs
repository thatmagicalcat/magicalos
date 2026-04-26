#![allow(static_mut_refs)]

use core::fmt::Debug;

use crate::{
    dbg_println,
    fd::FileDescriptor,
    fs::tar::{TarEntiresIterator, TarEntry},
    io::{self},
    limine_requests, scheduler,
};

mod data_handle;
mod error;
mod file;
pub mod tar;
mod vfs;

use vfs::*;

pub use file::*;
pub use vfs::VfsNodeId;

lazy_static::lazy_static! {
    pub static ref VFS: Vfs = Vfs::new();
}

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

pub fn init_ramfs() {
    log::info!("Initializing VFS...");

    let root = VFS.get_root_node_id();
    let ramfs = load_ramfs();

    for entry in ramfs {
        match entry {
            TarEntry::File { name, data } => {
                log::debug!(
                    "init_vfs(): mounting {} (size: {} bytes)",
                    &name[1..],
                    data.len()
                );

                VFS.mount(root, &name[1..], data).expect("failed to mount");
            }

            TarEntry::Directory { name: "./" } => {}
            TarEntry::Directory { name } => {
                log::debug!("init_vfs(): creating directory {}", &name[1..]);
                VFS.mkdir(root, &name[1..])
                    .expect("failed to create directory");
            }

            TarEntry::Other { .. } => log::error!("Other tar entry types are not supported yet :("),
        }
    }

    log::info!("Kernel VFS Tree:");

    dbg_println!("/");
    VFS.tree_lsdir(root).unwrap();
}

pub fn open(path: &str, options: OpenOptions) -> io::Result<FileDescriptor> {
    let cwd = scheduler::with_current_task(|task| task.cfg.get_cwd_id());
    scheduler::add_io_interface(VFS.open(cwd, path, options)?)
}

pub fn mkdir(path: &str) -> io::Result<()> {
    let cwd = scheduler::with_current_task(|task| task.cfg.get_cwd_id());
    VFS.mkdir(cwd, path).map(|_| ())
}

pub fn mount(path: &str, region: &'static [u8]) -> io::Result<()> {
    let cwd = scheduler::with_current_task(|task| task.cfg.get_cwd_id());
    VFS.mount(cwd, path, region).map(|_| ())
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
