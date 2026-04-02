use core::{mem, ops::Index, ptr};

use alloc::{collections::BTreeMap, vec::Vec};

use crate::graphics::{FrameBuffer, FrameBufferInfo, WindowConsole};

type PixelFormat = u32;

const PSF2_MAGIC: u32 = 0x864ab572;
const PSF2_HAS_UNICODE_TABLE: u32 = 0x01;
const PSF2_SEPARATOR: u8 = 0xFF;

#[derive(Debug, Clone, Copy)]
pub enum PSF2Error {
    InvalidMagic,
    DataTooSmall,
    InvalidHeaderSize,
    InvalidUTF8,
    InvalidVersion,
    TruncatedGlyphData,
    InvalidGlyphData,
    InvalidUnicodeTable,
}

impl core::error::Error for PSF2Error {}
impl core::fmt::Display for PSF2Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Invalid PSF2 magic number"),
            Self::InvalidUTF8 => write!(f, "Invalid UTF-8 in PSF2 unicode table"),
            Self::InvalidHeaderSize => write!(f, "Invalid PSF2 header size"),
            Self::TruncatedGlyphData => write!(f, "PSF2 glyph data is truncated"),
            Self::InvalidVersion => write!(f, "Unsupported version"),
            Self::DataTooSmall => write!(f, "PSF2 data is too small for header"),
            Self::InvalidGlyphData => write!(f, "PSF2 glyph data is invalid"),
            Self::InvalidUnicodeTable => write!(f, "PSF2 unicode table is invalid"),
        }
    }
}

#[repr(C)]
pub struct PSF2Header {
    /// magic bytes to identify PSF
    pub magic: u32,
    /// zero
    pub version: u32,
    /// offset of bitmaps in file, 32
    pub headersize: u32,
    /// 0 if there's no unicode table
    pub flags: u32,
    /// number of glyphs
    pub numglyph: u32,
    /// size of each glyph
    pub bytes_per_glyph: u32,
    /// height in pixels
    pub height: u32,
    /// width in pixels
    pub width: u32,
}

/// Parsed PSF2 font with Unicode mapping
#[derive(Debug, Clone)]
pub struct PSF2Font {
    pub width: u32,
    pub height: u32,
    // Raw glyph bitmap data
    pub glyph_bytes: Vec<u8>,
    // Bytes per glyph
    pub bytes_per_glyph: usize,
    // Unicode char -> glyph index
    pub unicode_map: BTreeMap<char, usize>,
}

impl PSF2Font {
    pub fn new(data: &[u8]) -> Result<Self, PSF2Error> {
        const HEADER_SIZE: usize = mem::size_of::<PSF2Header>();

        if data.len() < HEADER_SIZE {
            return Err(PSF2Error::DataTooSmall);
        }

        let header = unsafe { ptr::read_unaligned(data.as_ptr() as *const PSF2Header) };

        if header.magic != PSF2_MAGIC {
            return Err(PSF2Error::InvalidMagic);
        }

        if header.version != 0 {
            return Err(PSF2Error::InvalidMagic);
        }

        if header.headersize as usize != HEADER_SIZE {
            return Err(PSF2Error::InvalidHeaderSize);
        }

        let glyph_data_size = header.numglyph * header.bytes_per_glyph;

        if data.len() < HEADER_SIZE + glyph_data_size as usize {
            return Err(PSF2Error::TruncatedGlyphData);
        }

        // copy glyph data
        let glyphs = data[HEADER_SIZE..HEADER_SIZE + glyph_data_size as usize].to_vec();
        let mut unicode_map = BTreeMap::new();

        if header.flags & PSF2_HAS_UNICODE_TABLE != 0 {
            unicode_map = Self::parse_unicode_table(
                &data[HEADER_SIZE + glyph_data_size as usize..],
                header.numglyph as usize,
            )?;
        }

        Ok(Self {
            width: header.width,
            height: header.height,
            glyph_bytes: glyphs,
            bytes_per_glyph: header.bytes_per_glyph as usize,
            unicode_map,
        })
    }

    fn parse_unicode_table(
        data: &[u8],
        glyph_count: usize,
    ) -> Result<BTreeMap<char, usize>, PSF2Error> {
        let mut map = BTreeMap::new();
        let mut pos = 0;
        let mut glyph_index = 0;

        while pos < data.len() && glyph_index < glyph_count {
            // Read UTF-8 sequences until separator
            loop {
                if pos >= data.len() {
                    break;
                }

                if data[pos] == PSF2_SEPARATOR {
                    pos += 1;

                    if pos < data.len() && data[pos] == PSF2_SEPARATOR {
                        return Ok(map);
                    }

                    break;
                }

                // Decode UTF-8 character
                let ch = Self::decode_utf8(&data[pos..])?;
                pos += ch.len_utf8(); // bytes read

                // Map character to glyph index
                map.insert(ch, glyph_index);
            }

            glyph_index += 1;
        }

        Ok(map)
    }

    /// Decode a single UTF-8 character
    fn decode_utf8(data: &[u8]) -> Result<char, PSF2Error> {
        let end = data
            .iter()
            .position(|&b| b == PSF2_SEPARATOR)
            .unwrap_or(data.len());
        let slice = &data[..end];

        if slice.is_empty() {
            return Err(PSF2Error::TruncatedGlyphData);
        }

        core::str::from_utf8(slice)
            .map_err(|_| PSF2Error::InvalidUTF8)?
            .chars()
            .next()
            .ok_or(PSF2Error::TruncatedGlyphData)
    }

    pub fn get_glyph(&self, ch: char) -> Option<&[u8]> {
        let idx = self.unicode_map.get(&ch).copied().or_else(|| {
            let codepoint = ch as usize;
            if codepoint < self.glyph_count() {
                Some(codepoint)
            } else {
                None
            }
        })?;

        let start = idx * self.bytes_per_glyph;
        let end = start + self.bytes_per_glyph;

        Some(&self.glyph_bytes[start..end])
    }

    pub const fn glyph_count(&self) -> usize {
        self.glyph_bytes.len() / self.bytes_per_glyph
    }

    pub fn unicode_count(&self) -> usize {
        self.unicode_map.len()
    }

    /// x, y: Raw pixel coordinates for the top-left of the character
    pub fn write_char(
        &self,
        c: char,
        x: u32,
        y: u32,
        fg: u32,
        bg: Option<u32>,
        window_console: &WindowConsole,
    ) {
        let info = &window_console.info;
        let fb = &window_console.buffer;

        let glyph = self.get_glyph(c).unwrap_or_else(|| {
            self.get_glyph('?')
                .expect("Fallback glyph '?' not found in font")
        });

        let bytes_per_row = self.width.div_ceil(8);
        let bytes_per_pixel = info.bits_per_pixel as u32 / 8;

        let mut screen_row_byte_offset = (y * info.pitch) + (x * bytes_per_pixel);

        for font_y in 0..self.height {
            unsafe {
                let mut pixel_ptr = fb
                    .ptr
                    .add(screen_row_byte_offset as usize)
                    .cast::<PixelFormat>();

                let glyph_row_start = (font_y * bytes_per_row) as usize;
                let mut glyph_row_ptr = &glyph[glyph_row_start];
                let mut mask = 1 << 7;
                let mut current_byte = *glyph_row_ptr;

                for font_x in 0..self.width {
                    if current_byte & mask != 0 {
                        pixel_ptr.write_volatile(fg);
                    } else if let Some(bg) = bg {
                        pixel_ptr.write_volatile(bg);
                    };

                    pixel_ptr = pixel_ptr.add(1); // move to next pixel

                    mask >>= 1;
                    if mask == 0 && (font_x + 1) < self.width {
                        // move to next byte in glyph row
                        mask = 1 << 7;
                        glyph_row_ptr = &glyph[glyph_row_start + (font_x as usize + 1) / 8];
                        current_byte = *glyph_row_ptr;
                    }
                }

                screen_row_byte_offset += info.pitch; // move to next line on screen
            }
        }
    }
}

impl Index<char> for PSF2Font {
    type Output = [u8];

    fn index(&self, ch: char) -> &Self::Output {
        self.get_glyph(ch).expect("Glyph not found for character")
    }
}
