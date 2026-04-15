const ELF_OBJECT_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u16)]
#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
enum ObjectFileType {
    /// No file type
    None = 0,
    /// Relocatable file
    Relocatable = 1,
    /// Executable file
    Executable = 2,
    /// Shared object file
    Dynamic = 3,
    /// Core file
    Core = 4,
    /// Processor-specific
    LOPROC = 0xFF00,
    /// Processor-specific
    HIPROC = 0xFFFF,
}

#[repr(C, packed)]
#[derive(Debug)]
struct Identification {
    magic: [u8; 4],
    elf_class: ElfClass,
    data_encoding: ElfDataEncoding,
    /// current version is 1
    version: u8,
    _padding: [u8; 9],
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
enum ElfDataEncoding {
    /// Invalid
    None = 0,
    /// 2's complement values with least significant byte occupying the lowest address.
    TwosLSB,
    /// 2's complement values, with most significant byte occupying the lowest address
    TwosMsb,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
enum ElfClass {
    None = 0,
    Class32 = 1,
    Class64 = 2,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
#[allow(clippy::upper_case_acronyms)]
enum Elf64ProgramHeaderType {
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

#[derive(Debug)]
#[repr(C)]
struct Elf64ProgramHeader {
    type_: Elf64ProgramHeaderType,
    flags: u32,
    /// offset from the beginning of the file at which the first byte of this segment resides
    offset: u64,
    /// virtual address at which the first byte of the segment resides
    virual_address: u64,
    physical_address: u64,
    /// number of bytes in the file image of the segment; may be zero
    file_size: u64,
    /// number of bytes in the memory image of the segment; may be zero
    mem_size: u64,
    alignment: u64,
}

#[derive(Debug)]
#[repr(C)]
struct Elf64Header {
    /// The initial bytes mark the file as an object file and provide machine-independent
    /// data with which to decode and interpret the file's contents.
    ident: Identification,
    type_: ObjectFileType,
    machine: u16,
    /// must be 1
    version: u32,
    /// the virtual address to which the system first transfers control,
    /// thus starting the process. If the file has no associated entry point, this member holds
    /// zero.
    entry: u64,
    program_header_table_offset: u64,
    section_header_table_offset: u64,
    /// Processor-specific flags
    flags: u32,
    elf_header_size: u16,
    /// All entries are the same size.
    program_header_table_entry_size: u16,
    program_header_table_num_entires: u16,
    /// A section header is one entry in the section header table. All entries are the same size.
    section_header_size: u16,
    /// the product of this and `section_header_size` gives the section header table's size in
    /// bytes.
    section_header_table_entires: u16,
    /// This member holds the section header table index of the entry associated with the section
    /// name string table.
    section_header_string_table_index: u16,
}
