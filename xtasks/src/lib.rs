use std::path::Path;
use std::time::SystemTime;
use std::{fs, io};

/// Recursively find the newest modification time of a file or directory
pub fn get_newest_mtime(path: impl AsRef<Path>) -> io::Result<SystemTime> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;

    if metadata.is_file() {
        return metadata.modified();
    }

    let mut newest = metadata.modified()?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let mtime = get_newest_mtime(entry.path())?;
        if mtime > newest {
            newest = mtime;
        }
    }

    Ok(newest)
}

/// Recursively find the oldest modification time of a file or directory
pub fn get_oldest_mtime(path: impl AsRef<Path>) -> io::Result<SystemTime> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;

    if metadata.is_file() {
        return metadata.modified();
    }

    let mut oldest = metadata.modified()?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let mtime = get_oldest_mtime(entry.path())?;
        if mtime < oldest {
            oldest = mtime;
        }
    }

    Ok(oldest)
}

/// Returns true if any of the input files are newer than any of the output files
pub fn is_stale(inputs: &[&Path], outputs: &[&Path]) -> color_eyre::Result<bool> {
    let mut newest_input = SystemTime::UNIX_EPOCH;
    for input in inputs {
        if !input.exists() {
            continue;
        }
        let mtime = get_newest_mtime(input)?;
        if mtime > newest_input {
            newest_input = mtime;
        }
    }

    let mut oldest_output = SystemTime::now();
    for output in outputs {
        if !output.exists() {
            return Ok(true); // Output missing, definitely stale
        }
        let mtime = get_oldest_mtime(output)?;
        if mtime < oldest_output {
            oldest_output = mtime;
        }
    }

    Ok(newest_input > oldest_output)
}

pub fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to find project root")
        .to_path_buf()
}
