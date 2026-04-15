//! A tar parser

#[repr(C, packed)]
#[derive(Debug)]
struct TarHeader {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    chksum: [u8; 8],
    typeflag: u8,
    linkname: [u8; 100],
    magic: [u8; 6],
    version: [u8; 2],
    uname: [u8; 32],
    gname: [u8; 32],
    devmajor: [u8; 8],
    devminor: [u8; 8],
    prefix: [u8; 155],
}

fn parse_octal(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .take_while(|&&b| (b'0'..=b'7').contains(&b))
        .fold(0, |out, i| (out * 8) | (i - b'0') as u64)
}

pub struct TarEntiresIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> TarEntiresIterator<'a> {
    pub const fn new(data: &'a [u8]) -> TarEntiresIterator<'a> {
        TarEntiresIterator { data, offset: 0 }
    }
}

#[derive(Debug)]
pub enum TarEntry<'a> {
    File { name: &'a str, data: &'a [u8] },
    Directory { name: &'a str },
    Other { name: &'a str, typeflag: u8 },
}

impl<'a> Iterator for TarEntiresIterator<'a> {
    type Item = TarEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 512 > self.data.len() {
            return None;
        }

        let header_ptr = unsafe { self.data.as_ptr().add(self.offset) as *const TarHeader };
        let header = unsafe { &*header_ptr };

        // tar file ends with two empty 512-byte blocks
        if header.name[0] == 0 {
            return None;
        }

        let name = core::str::from_utf8(&header.name)
            .unwrap()
            .trim_matches('\0');
        let size = parse_octal(&header.size);
        let typeflag = header.typeflag;

        let out = match typeflag {
            b'0' | 0 => TarEntry::File {
                name,
                data: &self.data[self.offset + 512..self.offset + 512 + (size as usize)],
            },
            b'5' => TarEntry::Directory { name },
            _ => TarEntry::Other { name, typeflag },
        };

        self.offset += 512 + (size as usize).div_ceil(512) * 512;
        Some(out)
    }
}
