//! Checked search and mutation of flat, big-endian firmware images.

/// A signature or erased-space request.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Needle<'a> {
    /// Exact bytes; an empty pattern matches nothing.
    Bytes(&'a [u8]),
    /// A big-endian 32-bit value.
    Word(u32),
    /// A run of erased (`0xff`) bytes with an aligned start.
    FreeRun {
        /// Required number of bytes; zero matches nothing.
        len: usize,
        /// Start alignment in bytes; zero matches nothing.
        align: usize,
    },
    /// Big-endian `(value, mask)` words, matched at even offsets.
    Masked(&'a [(u16, u16)]),
}

/// Finds the first complete match at or after `start`.
/// Unaligned exact byte/word matches are permitted; masked instructions are
/// searched only at even offsets. A match does not prove a code boundary.
pub fn find(image: &[u8], needle: Needle<'_>, start: usize) -> Option<usize> {
    match needle {
        Needle::Bytes(pattern) => {
            if pattern.is_empty() {
                return None;
            }
            image
                .get(start..)?
                .windows(pattern.len())
                .position(|bytes| bytes == pattern)
                .map(|offset| start + offset)
        }
        Needle::Word(word) => find(image, Needle::Bytes(&word.to_be_bytes()), start),
        Needle::FreeRun { len, align } => find_free_space(image, len, align, start),
        Needle::Masked(pattern) => {
            if pattern.is_empty() {
                return None;
            }
            let len = pattern.len() * 2;
            let end = image.len().checked_sub(len)?;
            let start = start.checked_add(start % 2)?;
            (start..=end).step_by(2).find(|&at| {
                pattern.iter().enumerate().all(|(index, &(value, mask))| {
                    let word =
                        u16::from_be_bytes([image[at + index * 2], image[at + index * 2 + 1]]);
                    word & mask == value & mask
                })
            })
        }
    }
}

/// Finds an erased (`0xff`) run with its start aligned during the search.
/// Zero length/alignment and overflowing or out-of-bounds requests return `None`.
pub fn find_free_space(image: &[u8], len: usize, align: usize, start: usize) -> Option<usize> {
    if len == 0 || align == 0 {
        return None;
    }
    let end = image.len().checked_sub(len)?;
    let remainder = start % align;
    let start = start.checked_add(if remainder == 0 { 0 } else { align - remainder })?;
    (start..=end)
        .step_by(align)
        .find(|&at| image[at..at + len].iter().all(|&byte| byte == 0xff))
}

/// Reads a byte, or returns `None` when outside the image.
pub fn read_u8(image: &[u8], at: usize) -> Option<u8> {
    image.get(at).copied()
}

/// Reads a complete big-endian word without overflowing the input offset.
pub fn read_u16(image: &[u8], at: usize) -> Option<u16> {
    let bytes = image.get(at..)?.get(..2)?;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

/// Reads a complete big-endian longword without overflowing the input offset.
pub fn read_u32(image: &[u8], at: usize) -> Option<u32> {
    let bytes = image.get(at..)?.get(..4)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Why a checked image edit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    /// The complete edit does not fit in the image.
    OutOfBounds,
    /// An insertion would overwrite bytes other than erased flash (`0xff`).
    NotFree,
}

/// Overwrites bytes only after the complete range is checked.
/// Errors leave the image unchanged. Empty writes at the end are permitted.
pub fn write(image: &mut [u8], at: usize, bytes: &[u8]) -> Result<(), ImageError> {
    let destination = image
        .get_mut(at..)
        .and_then(|rest| rest.get_mut(..bytes.len()))
        .ok_or(ImageError::OutOfBounds)?;
    destination.copy_from_slice(bytes);
    Ok(())
}

/// Inserts into erased space, returning the flat-image offset on success.
/// Bounds and the whole erased range are checked before any byte is written.
pub fn insert(image: &mut [u8], at: usize, bytes: &[u8]) -> Result<usize, ImageError> {
    let destination = image
        .get_mut(at..)
        .and_then(|rest| rest.get_mut(..bytes.len()))
        .ok_or(ImageError::OutOfBounds)?;
    if destination.iter().any(|&byte| byte != 0xff) {
        return Err(ImageError::NotFree);
    }
    destination.copy_from_slice(bytes);
    Ok(at)
}

/// Geometry of fixed-size opcode-dispatch records with big-endian pointers.
#[derive(Debug, Clone, Copy)]
pub struct CommandTable {
    /// Flat-image offset of the first record.
    pub base: usize,
    /// Record size in bytes.
    pub stride: usize,
    /// Offset of the opcode byte inside each record.
    pub opcode_off: usize,
    /// Offset of the flags byte inside each record.
    pub flags_off: usize,
    /// Offset of the four-byte big-endian handler inside each record.
    pub handler_off: usize,
    /// Flags byte marking the terminating record (not returned by the scan).
    pub term_flag: u8,
    /// Maximum number of records to inspect, including the terminator.
    pub max_records: usize,
}

/// One dispatch record, with its pointer retained exactly as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandRecord {
    /// Record offset in the image.
    pub off: usize,
    /// Dispatch opcode.
    pub opcode: u8,
    /// Record flags.
    pub flags: u8,
    /// Big-endian handler value.
    pub handler: u32,
}

/// A malformed or incomplete command table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableError {
    /// Fields overlap, extend outside a record, or a size/count is zero.
    InvalidGeometry,
    /// A record is truncated or the table starts outside the image.
    OutOfBounds,
    /// No terminator was found within `max_records`.
    MissingTerminator,
    /// No record has the requested opcode.
    NotFound,
}

impl CommandTable {
    /// Reads all records up to a required terminator. No partial result is returned.
    pub fn walk(&self, image: &[u8]) -> Result<Vec<CommandRecord>, TableError> {
        let handler_end = self
            .handler_off
            .checked_add(4)
            .ok_or(TableError::InvalidGeometry)?;
        if self.stride == 0
            || self.max_records == 0
            || self.opcode_off >= self.stride
            || self.flags_off >= self.stride
            || handler_end > self.stride
            || self.opcode_off == self.flags_off
            || (self.handler_off..handler_end).contains(&self.opcode_off)
            || (self.handler_off..handler_end).contains(&self.flags_off)
        {
            return Err(TableError::InvalidGeometry);
        }
        let mut records = Vec::new();
        let mut at = self.base;
        for _ in 0..self.max_records {
            let record = image
                .get(at..)
                .and_then(|rest| rest.get(..self.stride))
                .ok_or(TableError::OutOfBounds)?;
            let flags = record[self.flags_off];
            if flags == self.term_flag {
                return Ok(records);
            }
            let pointer = &record[self.handler_off..handler_end];
            records.push(CommandRecord {
                off: at,
                opcode: record[self.opcode_off],
                flags,
                handler: u32::from_be_bytes([pointer[0], pointer[1], pointer[2], pointer[3]]),
            });
            at += self.stride; // the complete record was checked against image.len()
        }
        Err(TableError::MissingTerminator)
    }

    /// Finds the first matching opcode in a fully validated, terminated table.
    pub fn find(&self, image: &[u8], opcode: u8) -> Result<CommandRecord, TableError> {
        self.walk(image)?
            .into_iter()
            .find(|record| record.opcode == opcode)
            .ok_or(TableError::NotFound)
    }

    /// Changes one handler and optionally its flags, after validating the table.
    /// Errors leave the entire image unchanged.
    pub fn replace(
        &self,
        image: &mut [u8],
        opcode: u8,
        handler: u32,
        flags: Option<u8>,
    ) -> Result<(), TableError> {
        let record = self.find(image, opcode)?;
        let at = record.off + self.handler_off;
        image[at..at + 4].copy_from_slice(&handler.to_be_bytes());
        if let Some(flags) = flags {
            image[record.off + self.flags_off] = flags;
        }
        Ok(())
    }
}
