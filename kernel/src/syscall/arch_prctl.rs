use crate::{arch::msr, errno};

const ARCH_SET_FS: i32 = 0x1002;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_arch_prctl(code: i32, addr: usize) -> isize {
    log::trace!("Enter sys_arch_prctl: code={}, addr={:#x}", code, addr);

    if code == ARCH_SET_FS {
        unsafe {
            msr::wrmsr(msr::IA32_FS_BASE, addr as _);
        }

        return 0
    } 

    log::error!("Unimplemented arch_prctl code: {}", code);
    -errno::ENOSYS as _
}
