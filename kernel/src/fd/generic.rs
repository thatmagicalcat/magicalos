use alloc::string::String;

use crate::{
    drivers,
    io::{self, IoInterface},
};

#[derive(Debug)]
pub(crate) struct GenericStdin;

impl IoInterface for GenericStdin {}

#[derive(Debug)]
pub(crate) struct GenericStdout;

impl IoInterface for GenericStdout {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        // if log::log_enabled!(log::Level::Debug) {
        //     let s = unsafe { String::from_raw_parts(buf.as_ptr() as *mut _, buf.len(), buf.len()) };
        //     log::debug!("write(generic_stdout): {}", s.trim_end_matches(['\n', '\r']));
        //     core::mem::forget(s);
        // }

        drivers::terminal::TERMINAL
            .lock()
            .as_mut()
            .unwrap()
            .write_bytes(buf);

        Ok(buf.len())
    }
}

#[derive(Debug)]
pub(crate) struct GenericStderr;

impl IoInterface for GenericStderr {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let s = unsafe { String::from_raw_parts(buf.as_ptr() as *mut _, buf.len(), buf.len()) };
        log::error!("write(generic_stderr): {s}");
        core::mem::forget(s);

        drivers::terminal::TERMINAL
            .lock()
            .as_mut()
            .unwrap()
            .write_bytes(buf);
        Ok(buf.len())
    }
}
