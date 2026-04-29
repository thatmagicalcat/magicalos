use crate::arch::hpet::HPET;

/// TODO: add different clock types
#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_clock(sec_ptr: *mut i64, nsec_ptr: *mut i64) -> isize {
    if let Some(hpet) = HPET.get() {
        let uptime = hpet.uptime_nanos();
        let secs = (uptime / 1_000_000_000) as i64;
        let nanos = (uptime % 1_000_000_000) as i64;

        unsafe {
            if !sec_ptr.is_null() {
                *sec_ptr = secs;
            }

            if !nsec_ptr.is_null() {
                *nsec_ptr = nanos;
            }
        }

        0
    } else {
        -1 // HPET not initialized
    }
}
