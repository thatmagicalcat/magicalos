//! source: https://wiki.osdev.org/Global_Descriptor_Table

static GDT: [GdtEntry; 3] = [
    GdtEntry::empty(),
    GdtEntry::new_code(),
    GdtEntry::new_data(),
];

// source: https://wiki.osdev.org/GDT_Tutorial#:~:text=reloadSegments%3A%20%3B%20Reload,RET
pub fn init() {
    let gdtR = GdtR {
        size: (core::mem::size_of_val(&GDT) - 1) as _,
        base: GDT.as_ptr() as _,
    };

    unsafe {
        core::arch::asm! {
            "lgdt [{0}]",

            "push 8",
            "lea {1}, [2f]",
            "push {1}",
            "retfq",

            "2:",
            "mov ax, 16",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",

            in(reg) &gdtR,
            out(reg) _,
            out("ax") _,

            options(readonly, preserves_flags)
        }
    };
}

#[repr(C, packed)]
struct GdtR {
    /// size of table in bytes - 1
    pub size: u16,
    /// virtual address to the table
    pub base: u64,
}

struct GdtEntry(pub u64);
impl GdtEntry {
    const fn empty() -> Self {
        Self(0)
    }

    const fn new_code() -> Self {
        Self((1 << 44) | (1 << 47) | (1 << 41) | (1 << 43) | (1 << 53))
    }

    const fn new_data() -> Self {
        Self((1 << 44) | (1 << 47) | (1 << 41))
    }
}
