use core::time::Duration;

use crate::arch::hpet::HPET;

#[unsafe(no_mangle)]
pub extern "C" fn sys_sleep(secs: u64, nanos: u32) -> isize {
    if let Some(hpet) = HPET.get() {
        let duration = Duration::new(secs, nanos);
        hpet.busy_wait(duration);
        0 
    } else {
        -1 // HPET not initialized
    }
}
