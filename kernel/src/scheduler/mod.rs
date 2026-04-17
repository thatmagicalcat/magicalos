#![allow(static_mut_refs)]

use alloc::{rc::Rc, sync::Arc};
use core::cell::{RefCell, UnsafeCell};

use crate::{
    arch::interrupts,
    fd::FileDescriptor,
    io::{self, IoInterface},
    memory::paging::{PhysicalAddress, VirtualAddress},
    scheduler::{
        sched::Scheduler,
        task::{Task, TaskId, TaskPriority},
    },
};

mod sched;
mod task;

pub use task::*;

static mut SCHEDULER: Option<UnsafeCell<Scheduler>> = None;

pub(crate) fn init() {
    unsafe { SCHEDULER = Some(UnsafeCell::new(Scheduler::new())) };
}

pub fn spawn(f: extern "C" fn(), priority: TaskPriority) -> Result<TaskId, &'static str> {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).spawn(f, priority) }
}

pub fn reschedule() {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).reschedule() }
}

pub fn schedule() {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).schedule() }
}

pub fn exit() -> ! {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).exit() }
}

pub(crate) fn get_current_interrupt_stack() -> VirtualAddress {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).get_current_interrupt_stack() }
}

pub(crate) fn block_current_task() -> Rc<RefCell<Task>> {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).block_current_task() }
}

pub(crate) fn get_io_interface(fd: FileDescriptor) -> io::Result<Arc<dyn IoInterface>> {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).get_io_interface(fd) }
}

pub(crate) fn add_io_interface(interface: Arc<dyn IoInterface>) -> io::Result<FileDescriptor> {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).add_io_interface(interface) }
}

pub(crate) fn remove_io_interface(fd: FileDescriptor) -> io::Result<Arc<dyn IoInterface>> {
    interrupts::without_interrupts(|| unsafe {
        (*SCHEDULER.as_ref().unwrap().get()).remove_io_interface(fd)
    })
}

pub(crate) fn wakeup_task(task: &Rc<RefCell<Task>>) {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).wakeup_task(task) };
}

pub fn get_current_task_id() -> TaskId {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).get_current_task_id() }
}

pub(crate) fn set_root_page_table(physical_address: PhysicalAddress) {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).set_root_page_table(physical_address) }
}

pub(crate) fn get_root_page_table() -> PhysicalAddress {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).get_root_page_table() }
}
