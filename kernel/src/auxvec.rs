/// end of vector
pub const AT_NULL: u64 = 0;
/// entry should be ignored
pub const AT_IGNORE: u64 = 1;
/// file descriptor of program
pub const AT_EXECFD: u64 = 2;
/// program headers for program
pub const AT_PHDR: u64 = 3;
/// size of program header entry
pub const AT_PHENT: u64 = 4;
/// number of program headers
pub const AT_PHNUM: u64 = 5;
/// system page size
pub const AT_PAGESZ: u64 = 6;
/// base address of interpreter
pub const AT_BASE: u64 = 7;
/// flags
pub const AT_FLAGS: u64 = 8;
/// entry point of program
pub const AT_ENTRY: u64 = 9;
/// program is not ELF
pub const AT_NOTELF: u64 = 10;
/// real uid
pub const AT_UID: u64 = 11;
/// effective uid
pub const AT_EUID: u64 = 12;
/// real gid
pub const AT_GID: u64 = 13;
/// effective gid
pub const AT_EGID: u64 = 14;
/// string identifying CPU for optimizations
pub const AT_PLATFORM: u64 = 15;
/// arch dependent hints at CPU capabilities
pub const AT_HWCAP: u64 = 16;
/// frequency at which times() increments
pub const AT_CLKTCK: u64 = 17;
/// secure mode boolean
pub const AT_SECURE: u64 = 23;
/// string identifying real platform, may  differ from AT_PLATFORM.
pub const AT_BASE_PLATFORM: u64 = 24;
/// address of 16 random bytes
pub const AT_RANDOM: u64 = 25;
/// extension of AT_HWCAP
pub const AT_HWCAP2: u64 = 26;
/// filename of program
pub const AT_EXECFN: u64 = 31;
