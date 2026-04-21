pub const ELF_OBJECT_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u16)]
#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
pub enum ObjectFileType {
    /// No file type
    None = 0,
    /// Relocatable file
    Relocatable = 1,
    /// Absolute
    Executable = 2,
    /// Position independent
    Dynamic = 3,
    /// Core file
    Core = 4,
    /// Processor-specific
    LOPROC = 0xFF00,
    /// Processor-specific
    HIPROC = 0xFFFF,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Identification {
    pub magic: [u8; 4],
    pub elf_class: ElfClass,
    pub data_encoding: ElfDataEncoding,
    /// current version is 1
    pub version: u8,
    _padding: [u8; 9],
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ElfDataEncoding {
    /// Invalid
    None = 0,
    /// 2's complement values with least significant byte occupying the lowest address.
    TwosLSB,
    /// 2's complement values, with most significant byte occupying the lowest address
    TwosMsb,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ElfClass {
    None = 0,
    Class32 = 1,
    Class64 = 2,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
#[allow(clippy::upper_case_acronyms)]
pub enum Elf64ProgramHeaderType {
    Null = 0,
    /// Loadable segment
    Load = 1,
    /// Dynamic linking information
    Dynamic = 2,
    /// Interpreted program interpreter path name
    Interpreter = 3,
    /// Auxiliary information
    Note = 4,
    /// Reserved
    Shlib = 5,
    /// Program header table itself
    ProgramHeaderTable = 6,
    /// Processor-specific
    LOPROC = 0x70000000,
    /// Processor-specific
    HIPROC = 0x7FFFFFFF,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum Elf64ProgramHeaderFlag {
    Execute = 0x1,
    Write = 0x2,
    Read = 0x4
}

#[derive(Debug)]
#[repr(C)]
pub struct Elf64ProgramHeader {
    pub type_: Elf64ProgramHeaderType,
    pub flags: u32,
    /// offset from the beginning of the file at which the first byte of this segment resides
    pub offset: u64,
    /// virtual address at which the first byte of the segment resides
    pub virual_address: u64,
    pub physical_address: u64,
    /// number of bytes in the file image of the segment; may be zero
    pub file_size: u64,
    /// number of bytes in the memory image of the segment; may be zero
    pub mem_size: u64,
    pub alignment: u64,
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct Elf64Header {
    /// The initial bytes mark the file as an object file and provide machine-independent
    /// data with which to decode and interpret the file's contents.
    pub ident: Identification,
    pub type_: ObjectFileType,
    pub machine: u16,
    /// must be 1
    pub version: u32,
    /// the virtual address to which the system first transfers control,
    /// thus starting the process. If the file has no associated entry point, this member holds
    /// zero.
    pub entry: u64,
    pub program_header_table_offset: u64,
    pub section_header_table_offset: u64,
    /// Processor-specific flags
    pub flags: u32,
    pub elf_header_size: u16,
    /// All entries are the same size.
    pub program_header_table_entry_size: u16,
    pub program_header_table_num_entires: u16,
    /// A section header is one entry in the section header table. All entries are the same size.
    pub section_header_size: u16,
    /// the product of this and `section_header_size` gives the section header table's size in
    /// bytes.
    pub section_header_table_entires: u16,
    /// This member holds the section header table index of the entry associated with the section
    /// name string table.
    pub section_header_string_table_index: u16,
}
