#![allow(static_mut_refs)]

use alloc::rc::Rc;
use core::cell::{RefCell, UnsafeCell};

use crate::{
    memory::paging::VirtualAddress,
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

pub(crate) fn wakeup_task(task: &Rc<RefCell<Task>>) {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).wakeup_task(task) };
}

pub fn get_current_task_id() -> TaskId {
    unsafe { (*SCHEDULER.as_ref().unwrap().get()).get_current_task_id() }
}
