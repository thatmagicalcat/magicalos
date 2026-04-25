use alloc::{sync::Arc, vec::Vec};

use crate::{arch::processor, fs::File, io::Read, kernel, scheduler};

mod loader;
mod parser;

pub fn run(path: &str) {
    let mut elf_data = Vec::new();
    let mut file = File::open(path).expect("Failed to open user ELF file");
    file.read_to_end(&mut elf_data)
        .expect("Failed to read user ELF file");
    let elf_arc: Arc<[u8]> = elf_data.into();

    let (load_info, rsp) = scheduler::with_current_task(|task| {
        let load_info = loader::load_elf(task, &elf_arc);
        let mut stack = loader::StackBuilder::setup_user_stack(kernel::USER_STACK_TOP.0 as _);
        stack.create_auxv(&load_info, &task.cfg.argv, &task.cfg.envp);
        (load_info, stack.rsp)
    });

    log::info!("Leap of Faith!");
    unsafe { processor::jump_to_user_fn(load_info.entry, rsp) }
}
