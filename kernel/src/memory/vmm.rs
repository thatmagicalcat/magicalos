use core::{alloc::Layout, cmp::Ordering};

use alloc::collections::LinkedList;
use spin::Mutex;

use crate::{memory::paging::VirtualAddress, utils};

pub static KERNEL_VMM: Mutex<Vmm> = Mutex::new(Vmm::new());

pub const KERNEL_VMM_ADDR_START: usize = 0xFFFF900000000000;
pub const KERNEL_VMM_SIZE: usize = 0x1000000000; // 64 GiB

pub fn init_vmm() {
    KERNEL_VMM.lock().entries.push_back(VmmEntry {
        start: KERNEL_VMM_ADDR_START,
        end: KERNEL_VMM_ADDR_START + KERNEL_VMM_SIZE,
    });
}

pub struct Vmm {
    pub entries: LinkedList<VmmEntry>,
}

pub struct VmmEntry {
    pub start: usize,
    pub end: usize,
}

impl Vmm {
    pub const fn new() -> Self {
        Self {
            entries: LinkedList::new(),
        }
    }

    pub fn allocate(&mut self, layout: Layout) -> Result<VirtualAddress, &'static str> {
        log::trace!("vmm_allocate(): {layout:?}");

        let align = layout.align();
        let new_size = layout.size() + layout.align();
        let mut cursor = self.entries.cursor_front_mut();

        while let Some(node) = cursor.current() {
            let (region_start, region_size) = (node.start, node.end - node.start);

            match region_size.cmp(&new_size) {
                Ordering::Greater => {
                    let new_addr = VirtualAddress(utils::align_up(region_start as _, align) as _);
                    node.start += (new_addr.0 as usize - region_start) + layout.size();

                    // if there are any bytes left in the region, add a new entry for it
                    if new_addr.0 != region_start as _ {
                        cursor.insert_before(VmmEntry {
                            start: region_start,
                            end: new_addr.0 as usize,
                        });
                    }

                    return Ok(new_addr);
                }

                Ordering::Equal => {
                    let new_addr = VirtualAddress(utils::align_up(region_start as _, align) as _);

                    if new_addr.0 != region_start as _ {
                        node.end = new_addr.0 as usize;
                    } else {
                        cursor.remove_current();
                    }

                    return Ok(new_addr);
                }

                Ordering::Less => cursor.move_next(),
            }
        }

        Err("VMM: Out of virtual memory")
    }

    pub fn deallocate(&mut self, address: usize, size: usize) {
        let end = address + size;
        let mut cursor = self.entries.cursor_front_mut();

        while let Some(node) = cursor.current() {
            let (region_start, region_end) = (node.start, node.end);

            if region_start == end {
                node.start = address;

                if let Some(prev_node) = cursor.peek_prev() {
                    let prev_region_end = prev_node.end;

                    if prev_region_end == address {
                        prev_node.end = region_end;
                        cursor.remove_current();
                    }
                }

                return;
            } else if region_end == address {
                node.end = end;

                if let Some(next_node) = cursor.peek_next() {
                    let next_region_start = next_node.start;

                    if next_region_start == end {
                        next_node.start = region_start;
                        cursor.remove_current();
                    }
                }

                return;
            } else if end < region_start {
                let new_entry = VmmEntry {
                    start: address,
                    end,
                };

                cursor.insert_before(new_entry);
                return;
            }

            cursor.move_next();
        }

        let new_element = VmmEntry {
            start: address,
            end,
        };

        self.entries.push_back(new_element);
    }
}
