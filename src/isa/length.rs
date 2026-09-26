// Encoding references: H8/300 ADE-602-025 §2 instruction-format tables and
// Appendix A; H8/300H REJ09B0213 §2.4 Table 2.3; H8S REJ09B0139 §2.4 Table 2.2.
// The high-byte dispatch follows their operation-code maps.
use crate::Target;

fn word(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
    ]))
}

pub(super) fn recognize(bytes: &[u8], target: Target) -> Option<usize> {
    let w = word(bytes, 0)?;
    let hi = w >> 8;
    let lo = w & 255;
    let base = target == Target::H8_300;
    let s = matches!(target, Target::H8S2000 | Target::H8S2600);
    let mac = target == Target::H8S2600;
    let valid = match hi {
        0x00 => w == 0,
        0x01 => return prefix(bytes, target, w),
        0x02 | 0x03 => lo < 16 || (s && lo < 32) || (mac && lo & 0xe8 == 0x20),
        0x04..=0x08 | 0x0c | 0x0e | 0x14..=0x16 | 0x18 | 0x1c | 0x1e => true,
        0x09 | 0x0d | 0x19 | 0x1d => !base || lo & 0x88 == 0,
        0x0a | 0x1a => lo < 16 || (!base && lo & 0x88 == 0x80),
        0x0b | 0x1b => match lo >> 4 {
            0 | 8 => lo & 8 == 0,
            9 | 7 | 15 => !base && lo & 8 == 0,
            5 | 13 => !base,
            _ => false,
        },
        0x0f | 0x1f => lo < 16 || (!base && lo & 0x88 == 0x80),
        0x10..=0x13 => match lo >> 4 {
            0 | 8 => true,
            1 | 9 => !base,
            3 | 11 => !base && lo & 8 == 0,
            4 | 5 | 12 | 13 => s,
            7 | 15 => s && lo & 8 == 0,
            _ => false,
        },
        0x17 => match lo >> 4 {
            0 | 8 => true,
            1 | 5 | 9 | 13 => !base,
            3 | 7 | 11 | 15 => !base && lo & 8 == 0,
            _ => false,
        },
        0x20..=0x3f | 0x5b | 0x5f => true,
        // H8/300 §2 Bcc/BSR: the signed PC-relative displacement must
        // be even. The same even-destination requirement applies in
        // H8/300H and H8S (§2 branch instructions).
        0x40..=0x4f | 0x55 => lo & 1 == 0,
        0x50 | 0x51 => !base || lo & 8 == 0,
        0x52 | 0x53 => !base && lo & 8 == 0,
        0x54 | 0x56 => lo == 0x70,
        0x57 => !base && lo & 0xcf == 0,
        0x58 if !base && lo & 15 == 0 => return Some(4),
        0x59 | 0x5d => lo & 0x8f == 0,
        0x5a | 0x5e if !base || lo == 0 => return Some(4),
        0x5c if !base && lo == 0 => return Some(4),
        0x60..=0x63 | 0x67 | 0x68 | 0x6c => true,
        0x64..=0x66 => !base,
        0x69 | 0x6d => !base || lo & 8 == 0,
        0x6a | 0x6b => return absolute(bytes, target, w),
        0x6e => return Some(4),
        0x6f if !base || lo & 8 == 0 => return Some(4),
        0x70..=0x73 => lo & 0x80 == 0,
        0x74..=0x77 => true,
        0x78 if !base => return displacement(bytes, target, 0, w, false, false),
        0x79 if (base && lo < 8) || (!base && lo < 0x70) => return Some(4),
        0x7a if !base && lo < 0x70 && lo & 8 == 0 => return Some(6),
        0x7b if lo == 0x5c || (!base && lo == 0xd4) => {
            return if word(bytes, 2)? == 0x598f {
                Some(4)
            } else {
                None
            };
        }
        0x7c | 0x7d if lo & 0x8f == 0 => return bit_memory(bytes, 2, hi == 0x7d),
        0x7e | 0x7f => return bit_memory(bytes, 2, hi == 0x7f),
        0x80..=0xff => true,
        _ => false,
    };
    if valid {
        Some(2)
    } else {
        None
    }
}

// H8/300 §2 BAND through BXOR; H8S Table 2.2 pp.263–267. Read-only
// and read/modify/write prefixes have different permitted second opcodes.
fn bit_memory(bytes: &[u8], offset: usize, write: bool) -> Option<usize> {
    let w = word(bytes, offset)?;
    let valid = if write {
        matches!(w & 0xff0f, 0x6000 | 0x6100 | 0x6200 | 0x6700)
            || matches!(w & 0xff8f, 0x7000 | 0x7100 | 0x7200)
    } else {
        matches!(w & 0xff0f, 0x6300 | 0x7400 | 0x7500 | 0x7600 | 0x7700) || w & 0xff8f == 0x7300
    };
    if valid {
        Some(offset + 2)
    } else {
        None
    }
}

fn absolute(bytes: &[u8], target: Target, w: u16) -> Option<usize> {
    let base = target == Target::H8_300;
    match w & 0x00f0 {
        0x00 | 0x80 if !base || w & 0x0108 != 0x0108 => Some(4),
        0x40 | 0xc0 if w >> 8 == 0x6a => Some(4),
        0x20 | 0xa0 if !base => extension(bytes, target, 2, 6),
        0x10 | 0x30 if matches!(target, Target::H8S2000 | Target::H8S2600) && w & 0x0107 == 0 => {
            bit_memory(bytes, if w & 0x20 == 0 { 4 } else { 6 }, w & 8 != 0)
        }
        _ => None,
    }
}

// H8/300H Table 2.3: the first byte of a 24-bit EA extension is fixed
// zero. H8S Table 2.2 uses all 32 bits in that same four-byte slot.
fn extension(bytes: &[u8], target: Target, offset: usize, len: usize) -> Option<usize> {
    if target == Target::H8_300H && *bytes.get(offset)? != 0 {
        None
    } else {
        Some(len)
    }
}

fn displacement(
    bytes: &[u8],
    target: Target,
    offset: usize,
    a: u16,
    long: bool,
    control: bool,
) -> Option<usize> {
    let b = word(bytes, offset + 2)?;
    // H8S Table 2.2 note 2 permits bit 7 of MOV.L's fourth byte on stores.
    let store_alias = long && !control && b & 0xfff8 == 0x6ba0;
    if a & (if store_alias { 0xff0f } else { 0xff8f }) != 0x7800 {
        return None;
    }
    let mask = if control {
        0xff7f
    } else if long {
        0xff78
    } else {
        0xfe70
    };
    if b & mask != if long || control { 0x6b20 } else { 0x6a20 } {
        return None;
    }
    extension(bytes, target, offset + 4, offset + 8)
}

fn prefix(bytes: &[u8], target: Target, p: u16) -> Option<usize> {
    if p == 0x0180 {
        return Some(2);
    }
    if target == Target::H8_300 {
        return None;
    }
    let s = matches!(target, Target::H8S2000 | Target::H8S2600);
    if p == 0x01a0 && target == Target::H8S2600 {
        return Some(2);
    }
    let w = word(bytes, 2)?;
    let valid = match p {
        0x0100 => return prefixed_move(bytes, target, w, false),
        0x0140 => return prefixed_move(bytes, target, w, true),
        0x0141 if s => {
            if matches!(w >> 8, 4..=7) {
                return Some(4);
            }
            return prefixed_move(bytes, target, w, true);
        }
        // H8S §2.2.36 / §2.2.62: serial register groups, no wrap.
        0x0110 | 0x0120 | 0x0130 if s => {
            let count = (p >> 4) & 3;
            (w & 0xfff8 == 0x6d70 && w & 7 >= count)
                || (w & 0xfff8 == 0x6df0 && (w & 7) + count <= 7)
        }
        0x0160 if target == Target::H8S2600 => w & 0xff88 == 0x6d00,
        0x01c0 => w >> 8 == 0x50 || w & 0xff08 == 0x5200,
        0x01d0 => w >> 8 == 0x51 || w & 0xff08 == 0x5300,
        // H8S Table 2.2 note 3 restricts TAS to ER0, ER1, ER4, ER5.
        0x01e0 if s => w & 0xffaf == 0x7b0c,
        0x01f0 => matches!(w & 0xff88, 0x6400 | 0x6500 | 0x6600),
        _ => false,
    };
    if valid {
        Some(4)
    } else {
        None
    }
}

fn prefixed_move(bytes: &[u8], target: Target, w: u16, control: bool) -> Option<usize> {
    if w & if control { 15 } else { 8 } != 0 {
        return None;
    }
    match w >> 8 {
        0x69 | 0x6d => Some(4),
        0x6f => Some(6),
        0x6b => match w & 0xf0 {
            0 | 0x80 => Some(6),
            0x20 | 0xa0 => extension(bytes, target, 4, 8),
            _ => None,
        },
        0x78 => displacement(bytes, target, 2, w, true, control),
        _ => None,
    }
}
