use alloc::string::String;

use crate::{
    arch::interrupts,
    drivers,
    io::{self, IoInterface},
    scheduler,
};

#[derive(Debug)]
pub(crate) struct GenericStdin;

impl IoInterface for GenericStdin {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        loop {
            let mut stdin = drivers::keyboard::KYEBOARD_STATE.lock();

            if stdin.lines_ready > 0 {
                stdin.lines_ready -= 1;

                let mut bytes_read = 0;
                while bytes_read < buf.len() {
                    if let Some(ch) = stdin.ready_buffer.pop_front() {
                        buf[bytes_read] = ch;
                        bytes_read += 1;

                        if ch == b'\n' {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                return Ok(bytes_read);
            }

            stdin.add_waiter(scheduler::get_current_task_id());

            drop(stdin);

            log::trace!("Keyboard STDIN queue empty, blocking current thread");
            scheduler::block_current_task();
            scheduler::reschedule();
        }
    }
}

#[derive(Debug)]
pub(crate) struct GenericStdout;

impl IoInterface for GenericStdout {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        interrupts::without_interrupts(|| {
            drivers::terminal::TERMINAL
                .lock()
                .as_mut()
                .unwrap()
                .write_bytes(buf)
        });

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
