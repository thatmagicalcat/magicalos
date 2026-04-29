use crate::scheduler;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_exit() {
    log::trace!("Enter sysexit");
    scheduler::exit();
}
