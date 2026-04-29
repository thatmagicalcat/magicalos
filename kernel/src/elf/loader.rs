use core::ptr::copy_nonoverlapping;

use alloc::{ffi::CString, sync::Arc, vec::Vec};

use crate::{
    auxvec,
    elf::parser::*,
    kernel::{self, USER_ENTRY, USER_STACK_BOTTOM, USER_STACK_TOP},
    memory::{
        self,
        paging::{PageTableEntryFlags, VirtualAddress},
    },
    scheduler, utils,
};

pub struct LoadInfo {
    pub entry: usize,
    pub base_address: usize,
    pub ph_off: usize,
    pub ph_num: usize,
    pub ph_ent: usize,
}

pub struct StackBuilder {
    pub rsp: usize,
    pub hhdm_ptr: *mut u8,
    pub stack_page_addr: usize,
}

impl StackBuilder {
    pub fn setup_user_stack(usr_stack_top: usize) -> Self {
        let stack_page_addr = usr_stack_top - memory::PAGE_SIZE;
        let hhdm_offset = kernel::get_hhdm_offset();
        let frame = memory::allocate_frame().expect("oom");
        let hhdm_ptr = (frame.start_address() + hhdm_offset) as *mut u8;

        // zero out everything
        unsafe { core::ptr::write_bytes(hhdm_ptr, 0, memory::PAGE_SIZE) };

        memory::paging::PageTable::active().mapper_mut().map_to(
            VirtualAddress(stack_page_addr as _),
            frame,
            PageTableEntryFlags::USER_ACCESSIBLE
                | PageTableEntryFlags::WRITABLE
                | PageTableEntryFlags::NO_EXECUTE,
            &mut *memory::lock_global_frame_allocator(),
        );

        Self {
            rsp: usr_stack_top,
            hhdm_ptr,
            stack_page_addr,
        }
    }

    fn push_auxkv(&mut self, k: u64, v: u64) {
        self.push_qword(v);
        self.push_qword(k);
    }

    pub fn create_auxv(
        &mut self,
        &LoadInfo {
            entry,
            base_address,
            ph_off,
            ph_num,
            ph_ent,
        }: &LoadInfo,
        args: &[CString],
        envs: &[CString],
    ) {
        // 16 bytes of random data
        // TODO: generate this value using a PRNG
        let entropy: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ];

        let at_random_ptr = self.push_bytes(&entropy);

        let envp_ptrs = envs
            .iter()
            .map(|envp| self.push_bytes(envp.as_bytes()))
            .collect::<Vec<_>>();

        let argv_ptrs = args
            .iter()
            .map(|arg| self.push_bytes(arg.as_bytes_with_nul()))
            .collect::<Vec<_>>();

        /*
         * usr_stack_top somewhere around here
         *
         * High addresses
         * +----------------------------------------+
         * |         INFORMATION BLOCK              |
         * |      (actual raw string bytes)         |
         * +----------------------------------------+
         * | "PATH=/bin\0" <- e.g. of envp string 1 |
         * | "USER=cat"    <- e.g. of envp string 0 |
         * | "./usr.elf\0" <- e.g. of argv string 0 |
         * +----------------------------------------+
         * | 16 bytes of random data (AT_RANDOM)    |
         * +----------------------------------------+
         * | 0 to 15 bytes of padding               |
         * +----------------------------------------+
         * | auxv AT_NULL <- end of auxv            |
         * | auxv value                             |
         * | auxv key (AT_RANDOM)                   |
         * | ...                                    |
         * | auxv value (e.g. 4096)                 |
         * | auxv key (AT_PAGESZ) <- start of auxv  |
         * +----------------------------------------+
         * | NULL <- end of envp array              |
         * | envp pointers                          |
         * +----------------------------------------+
         * | NULL <- end of argv array              |
         * | argv pointers                          |
         * +----------------------------------------+
         * | argc <- argument count                 |
         * +----------------------------------------+
         * Low addresses (<- final rsp)
         * */

        // argc (1) + argv (x) + envp (y) + auxv (16) = 17 + x + y slots
        let final_rsp_unaligned = self.rsp - (1 + args.len() + envs.len() + 16) * 8;
        let final_rsp_aligned = utils::align_down(final_rsp_unaligned, 16);
        let padding = final_rsp_unaligned - final_rsp_aligned;

        self.rsp -= padding;

        self.push_auxkv(auxvec::AT_NULL, 0);
        self.push_auxkv(auxvec::AT_RANDOM, at_random_ptr as _);
        self.push_auxkv(auxvec::AT_ENTRY, entry as _);
        self.push_auxkv(auxvec::AT_BASE, base_address as _);
        self.push_auxkv(auxvec::AT_PAGESZ, memory::PAGE_SIZE as _);
        self.push_auxkv(auxvec::AT_PHNUM, ph_num as _);
        self.push_auxkv(auxvec::AT_PHENT, ph_ent as _);
        self.push_auxkv(auxvec::AT_PHDR, (ph_off + base_address) as _);

        self.push_qword(0); // ENVP NULL
        for p in envp_ptrs {
            self.push_qword(p as _);
        }

        self.push_qword(0); // ARGV NULL
        for p in argv_ptrs {
            self.push_qword(p as _);
        }

        // ARGC
        self.push_qword(args.len() as _);
    }

    /// Returns the user-space virtual address of the pushed u64
    fn push_qword(&mut self, v: u64) -> usize {
        self.rsp -= core::mem::size_of::<u64>();
        let offset = self.rsp - self.stack_page_addr;
        unsafe { core::ptr::write_unaligned(self.hhdm_ptr.add(offset).cast(), v) };

        self.rsp
    }

    /// Returns the user-space virtual address of pushed bytes
    fn push_bytes(&mut self, bytes: &[u8]) -> usize {
        self.rsp -= bytes.len();
        let offset = self.rsp - self.stack_page_addr;
        unsafe { copy_nonoverlapping(bytes.as_ptr(), self.hhdm_ptr.add(offset), bytes.len()) };

        self.rsp
    }
}

pub fn load_elf(task: &mut scheduler::Task, elf_data: &Arc<[u8]>) -> LoadInfo {
    if elf_data.len() < core::mem::size_of::<Elf64Header>() {
        panic!("ELF data too small");
    }

    let header = unsafe { core::ptr::read_unaligned(elf_data.as_ptr() as *const Elf64Header) };

    assert!(header.ident.magic == ELF_OBJECT_MAGIC);

    task.vmspace
        .insert(
            USER_STACK_BOTTOM.0 as _,
            USER_STACK_TOP.0 as _,
            PageTableEntryFlags::USER_ACCESSIBLE
                | PageTableEntryFlags::WRITABLE
                | PageTableEntryFlags::NO_EXECUTE,
            memory::MappingType::Anonymous,
        )
        .unwrap();

    let type_ = header.type_;
    let base_address: usize = if type_ == ObjectFileType::Dynamic {
        log::debug!("Dynamic ELF detected, using USER_ENTRY as base address");
        USER_ENTRY.0 as _
    } else {
        0
    };

    let entry = base_address + header.entry as usize;
    let ph_off = header.program_header_table_offset as usize;
    let ph_num = header.program_header_table_num_entires as usize;
    let ph_ent = header.program_header_table_entry_size as usize;

    for i in 0..ph_num {
        let offset = ph_off + i * ph_ent;
        let phdr: Elf64ProgramHeader = unsafe {
            core::ptr::read_unaligned(elf_data.as_ptr().add(offset) as *const Elf64ProgramHeader)
        };

        if phdr.type_ != Elf64ProgramHeaderType::Load {
            log::warn!("PHDR type: {:?} is not supported yet", phdr.type_);
            continue;
        }

        map_segment(base_address, elf_data, task, phdr);
    }

    LoadInfo {
        entry,
        base_address,
        ph_off,
        ph_num,
        ph_ent,
    }
}

fn map_segment(
    base_address: usize,
    elf_arc: &Arc<[u8]>,
    task: &mut scheduler::Task,
    phdr: Elf64ProgramHeader,
) {
    let vaddr = base_address + phdr.virual_address as usize;
    let mem_size = phdr.mem_size as usize;
    let file_size = phdr.file_size as usize;
    let file_offset = phdr.offset as usize;

    let misalign = vaddr % memory::PAGE_SIZE;

    let start_page = vaddr - misalign;
    let end_page = utils::align_up(vaddr + mem_size, memory::PAGE_SIZE);

    let aligned_file_offset = file_offset - misalign;
    let aligned_file_size = file_size + misalign;

    let mut flags = PageTableEntryFlags::USER_ACCESSIBLE;

    if phdr.flags & Elf64ProgramHeaderFlag::Write as u32 != 0 {
        flags |= PageTableEntryFlags::WRITABLE;
    }

    if phdr.flags & Elf64ProgramHeaderFlag::Execute as u32 == 0 {
        flags |= PageTableEntryFlags::NO_EXECUTE;
    }

    log::info!("ELF LOAD: {start_page:#X}..{end_page:#X}, {flags:?}");

    task.vmspace
        .insert(
            start_page,
            end_page,
            flags,
            memory::MappingType::Elf {
                data: Arc::clone(elf_arc),
                file_offset: aligned_file_offset,
                file_size: aligned_file_size,
            },
        )
        .expect("Failed to insert ELF VMA");
}
