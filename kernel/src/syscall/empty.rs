#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_empty() {
    log::warn!("Enter sys_empty")
}
