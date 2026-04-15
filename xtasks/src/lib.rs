use std::path::Path;
use std::process::ExitStatus;
use std::{fs, io};

pub type DynError = Box<dyn std::error::Error>;

pub trait EarlyRet<E>: Sized {
    fn early_ret(self) -> Result<Self, E>;
}

impl EarlyRet<DynError> for ExitStatus {
    fn early_ret(self) -> Result<Self, DynError> {
        match self.success() {
            true => Ok(self),
            false => Err(io::Error::other("Command failed").into()),
        }
    }
}

/// Copy all the contents of src directory to dst directory, creating dst if it doesn't exist
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    create_dir(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(
                dbg!(entry.path()),
                dbg!(dst.as_ref().join(entry.file_name())),
            )?;
        }
    }

    Ok(())
}

/// Creates a directory and all of its parent components if they are missing, but does nothing if
/// the directory already exists
pub fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    if !path.as_ref().exists() {
        fs::create_dir_all(path)?;
    }

    Ok(())
}
