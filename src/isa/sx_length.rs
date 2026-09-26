// H8SX REJ09B0102 §2.4 (Table 2.2). This is a deliberately narrow
// recognizer: an older core's length rule cannot safely be reused for H8SX.

fn word(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
    ]))
}

pub(super) fn recognize(bytes: &[u8]) -> Option<usize> {
    let first = word(bytes, 0)?;
    match first {
        // Operation-only rows: NOP, SLEEP, RTS, RTE.
        0x0000 | 0x0180 | 0x5470 | 0x5670 => Some(2),
        // ADD.B Rs,Rd and MOV.B Rs,Rd; both register fields occupy a
        // nibble. MOV.B @aa:8,Rd and MOV.B Rs,@aa:8 use one nibble for
        // the byte register and one byte for the address (H8SX §2.4).
        first if matches!(first >> 8, 0x08 | 0x0c | 0x20..=0x3f) => Some(2),
        // H8SX §2.4 Bcc: d:8 occupies seven bits and bit 0 is zero.
        // 40xx with bit 0 set is instead BRA/S, which has a delay slot
        // (§2.2.24) but is still one two-byte instruction.
        first if first >> 8 == 0x40 => Some(2),
        first if (0x41..=0x4f).contains(&(first >> 8)) && first & 1 == 0 => Some(2),
        // Bcc d:16, BSR d:8, and BSR d:16 (H8SX §2.4).
        first if first & 0xff0f == 0x5800 => Some(4),
        first if first >> 8 == 0x55 => Some(2),
        0x5c00 => Some(4),
        // Eight immediate-byte/register rows: ADD, ADDX, CMP, SUBX,
        // OR, XOR, AND and MOV (H8SX §2.4 Table 2.2). The high nibble
        // selects the operation, the next nibble a byte register, and
        // the low byte is the immediate value.
        first if first >= 0x8000 => Some(2),
        // ADD.B @(d:32,ERs),<destination>, manual page 640 (PDF page 658).
        // Opcode words: 780rs0100, 6a2c, then a destination-mode word.
        // The four-byte source displacement follows 6a2c. The destination
        // word may be followed by a two- or four-byte EA extension.
        first if first & 0xff8f == 0x7804 => {
            let second = word(bytes, 2)?;
            if second != 0x6a2c {
                return None;
            }
            let third = word(bytes, 8)?;
            match third & 0xf8ff {
                // @ERd, @ERd+, @ERd-, @+ERd, @-ERd.
                0x0010 | 0x8010 | 0xa010 | 0x9010 | 0xb010 => Some(10),
                // @(d:16,ERd), the three 16-bit indexed forms.
                0xc010 | 0xd010 | 0xe010 | 0xf010 => Some(12),
                // @(d:32,ERd), the three 32-bit indexed forms.
                0xc810 | 0xd810 | 0xe810 | 0xf810 => Some(14),
                // Absolute addresses have no destination register field.
                0x4010 if third == 0x4010 => Some(12),
                0x4810 if third == 0x4810 => Some(14),
                _ => None,
            }
        }
        _ => None,
    }
}
