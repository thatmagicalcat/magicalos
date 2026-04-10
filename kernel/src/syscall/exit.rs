use crate::scheduler;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_exit() {
    log::debug!("Enter sysexit");
    scheduler::exit();
}
