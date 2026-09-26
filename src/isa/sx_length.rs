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
        // STC.B CCR/EXR,Rd and LDC.B Rs,CCR/EXR. Bit 4 selects the
        // control register; the low nibble selects a byte register.
        first if matches!(first >> 8, 0x02 | 0x03) && first & 0x00e0 == 0 => Some(2),
        // STMAC/STC.L and LDMAC/LDC.L use three-bit ER fields; bit 3
        // remains zero. The high byte selects store (02) or load (03).
        first
            if matches!(first >> 8, 0x02 | 0x03)
                && matches!(first & 0x00f8, 0x20 | 0x30 | 0x60 | 0x70) =>
        {
            Some(2)
        }
        // H8SX SHLL/SHLR #xx:5 register forms. The count occupies
        // the low five bits of 038x/039x and must be 1..31; the second
        // word selects B/W/L size and destination register (§2.4).
        first if first & 0xffe0 == 0x0380 && first & 0x001f != 0 => {
            let second = word(bytes, 2)?;
            match second {
                word if matches!(word >> 8, 0x10 | 0x11) && word & 0x00f0 < 0x0020 => Some(4),
                word if matches!(word >> 8, 0x10 | 0x11) && word & 0x00f8 == 0x0030 => Some(4),
                _ => None,
            }
        }
        // ORC, XORC, ANDC and LDC.B #xx:8,CCR. The low byte is the
        // immediate, so all 256 variants of each opcode are allocated.
        first if matches!(first >> 8, 0x04..=0x07) => Some(2),
        // LDM.L/STM.L transfer two to four consecutive ER registers.
        // The 01n0 prefix encodes the group size minus one; 6d7r names
        // the last loaded register, while 6dfr names the first stored
        // register (H8SX §2.4 LDM/STM rows).
        first if matches!(first, 0x0110 | 0x0120 | 0x0130) => {
            let second = word(bytes, 2)?;
            let count_minus_one = (first >> 4) as u8 & 3;
            let register = (second & 7) as u8;
            match second & 0xfff8 {
                0x6d70 if register >= count_minus_one => Some(4),
                0x6df0 if register + count_minus_one < 8 => Some(4),
                _ => None,
            }
        }
        // LDC.W/STC.W transfer CCR or EXR through memory. The prefix
        // low bit selects EXR; 0141 07xx loads an immediate byte into
        // EXR. Memory forms reserve the second word's low nibble.
        first if matches!(first, 0x0140 | 0x0141) => {
            let second = word(bytes, 2)?;
            match second {
                word if first == 0x0141 && word >> 8 == 0x07 => Some(4),
                word if matches!(word >> 8, 0x69 | 0x6d) && word & 0x000f == 0 => Some(4),
                word if word >> 8 == 0x6f && word & 0x000f == 0 => Some(6),
                0x6b00 | 0x6b80 => Some(6),
                0x6b20 | 0x6ba0 => Some(8),
                _ => None,
            }
        }
        // MAC @ERn+,@ERm+ has two three-bit ER fields (H8SX §2.4).
        0x0160 => {
            let second = word(bytes, 2)?;
            if second & 0xff88 == 0x6d00 {
                Some(4)
            } else {
                None
            }
        }
        // MOV.L single-memory-operand forms (H8SX §2.4 MOV rows).
        // The 0100 prefix is followed by a second opcode word. Only
        // the exact register/EA families below are recognized here.
        0x0100 => {
            let second = word(bytes, 2)?;
            match second {
                word if matches!(word >> 8, 0x69 | 0x6d) && word & 8 == 0 => Some(4),
                word if word >> 8 == 0x6f && word & 8 == 0 => Some(6),
                word if matches!(word & 0xfff8, 0x6b00 | 0x6b80) => Some(6),
                word if matches!(word & 0xfff8, 0x6b20 | 0x6ba0) => Some(8),
                _ => None,
            }
        }
        // ADD.B Rs,Rd and MOV.B Rs,Rd; both register fields occupy a
        // nibble. MOV.B @aa:8,Rd and MOV.B Rs,@aa:8 use one nibble for
        // the byte register and one byte for the address (H8SX §2.4).
        first if matches!(first >> 8, 0x08 | 0x0c | 0x0e | 0x14..=0x16 | 0x18 | 0x1c | 0x1e | 0x20..=0x3f) => {
            Some(2)
        }
        // Word register pairs use full four-bit R/E register fields:
        // ADD, MOV, CMP, SUB, OR, XOR, AND (H8SX §2.4 Table 2.2).
        first if matches!(first >> 8, 0x09 | 0x0d | 0x19 | 0x1d | 0x64..=0x66) => Some(2),
        // BSET/BNOT/BCLR/BTST Rn,Rd and BST/BIST Rn,Rd. Both low-byte
        // nibbles are register fields (H8SX §2.4 bit-operation rows).
        first if matches!(first >> 8, 0x60..=0x63 | 0x67) => Some(2),
        // The immediate bit number occupies bits 6–4; bit 7 is fixed
        // zero in the register-destination rows (H8SX §2.4).
        first if matches!(first >> 8, 0x70..=0x73) && first & 0x0080 == 0 => Some(2),
        first if matches!(first >> 8, 0x74..=0x77) => Some(2),
        // ADD/MOV/SUB/CMP .W #xx:3,Rd use low-byte bits 6–4 for the
        // immediate and bits 3–0 for Rd. The .L register-pair forms
        // set bit 7 and clear bit 3; .L #xx:3,ERd sets both bits and
        // reserves immediate zero (§2.4 arithmetic/move rows).
        first
            if matches!(first >> 8, 0x0a | 0x0f | 0x1a | 0x1f)
                && (first & 0x0080 == 0
                    || first & 0x0088 == 0x0080
                    || first & 0x0088 == 0x0088 && first & 0x0070 != 0) =>
        {
            Some(2)
        }
        // ADDS/SUBS #1/#2/#4,ERd and INC/DEC .W/.L #1/#2.
        // The ER destination uses three bits; the word destination
        // uses four (H8SX §2.4 ADDS/SUBS/INC/DEC rows).
        first if matches!(first >> 8, 0x0b | 0x1b) => match first & 0x00f8 {
            0x00 | 0x80 | 0x90 | 0x70 | 0xf0 => Some(2),
            0x50 | 0x58 | 0xd0 | 0xd8 => Some(2),
            _ => None,
        },
        // SHLL/SHLR/SHAL/SHAR and ROTXL/ROTXR/ROTL/ROTR register
        // forms. Byte/word registers use four bits; ER destinations use
        // three. H8SX §2.4 also allocates shift counts 2/4/8/16 where
        // listed in the corresponding operation rows.
        first if matches!(first >> 8, 0x10 | 0x11) => {
            let low = first as u8;
            if matches!(low >> 4, 0x0..=0xa | 0xc | 0xd | 0xf) || low >> 4 == 0xb && low & 8 == 0 {
                Some(2)
            } else {
                None
            }
        }
        first if matches!(first >> 8, 0x12 | 0x13) => {
            let low = first as u8;
            if matches!(low >> 4, 0 | 1 | 4 | 5 | 8 | 9 | 12 | 13)
                || matches!(low >> 4, 3 | 7 | 11 | 15) && low & 8 == 0
            {
                Some(2)
            } else {
                None
            }
        }
        // NOT, NEG, EXTU and EXTS register forms. Byte/word operands
        // use a four-bit register; long operands require bit 3 clear.
        // H8SX §2.4 fixes the additional bits in EXTU.L/EXTS.L, even
        // though binutils decodes some aliases in this group.
        first if first >> 8 == 0x17 => {
            let low = first as u8;
            if matches!(low >> 4, 0 | 1 | 5 | 8 | 9 | 13)
                || matches!(low >> 4, 3 | 7 | 11 | 15) && low & 8 == 0
            {
                Some(2)
            } else {
                None
            }
        }
        // MOV.B/W register-indirect, post-increment/pre-decrement, and
        // 16-bit displacement forms. Bit 7 selects transfer direction;
        // the remaining nibbles are register fields (H8SX §2.4).
        first if matches!(first >> 8, 0x68 | 0x69 | 0x6c | 0x6d) => Some(2),
        first if matches!(first >> 8, 0x6e | 0x6f) => Some(4),
        // MOV.B/W @aa:16/@aa:32 load and store. The high nibble of
        // the low byte selects transfer direction and address width;
        // the low nibble names the data register (H8SX §2.4 MOV rows).
        first if matches!(first >> 8, 0x6a | 0x6b) => match first & 0x00f0 {
            0x00 | 0x80 => Some(4),
            0x20 | 0xa0 => Some(6),
            // Six byte-immediate arithmetic/logical operations on an
            // absolute memory byte. The EA extension precedes the
            // 8x/ax/cx/dx/ex opcode-immediate word (§2.4).
            0x10 | 0x30 if matches!(first, 0x6a18 | 0x6a38) => {
                let opcode = word(bytes, if first == 0x6a18 { 4 } else { 6 })?;
                if matches!(opcode >> 8, 0x80 | 0xa0 | 0xa1 | 0xc0 | 0xd0 | 0xe0) {
                    Some(if first == 0x6a18 { 6 } else { 8 })
                } else {
                    None
                }
            }
            _ => None,
        },
        // H8SX §2.4 Bcc: d:8 occupies seven bits and bit 0 is zero.
        // 40xx with bit 0 set is instead BRA/S, which has a delay slot
        // (§2.2.24) but is still one two-byte instruction.
        first if first >> 8 == 0x40 => Some(2),
        first if (0x41..=0x4f).contains(&(first >> 8)) && first & 1 == 0 => Some(2),
        // Unsigned byte multiply/divide use two full register nibbles.
        // The word forms name an ER destination in only three bits.
        first if matches!(first >> 8, 0x50 | 0x51) => Some(2),
        first if matches!(first >> 8, 0x52 | 0x53) && first & 8 == 0 => Some(2),
        // TRAPA #x:2 occupies low-byte bits 5–4; all other bits are zero.
        first if first >> 8 == 0x57 && first & 0x00cf == 0 => Some(2),
        // Bcc d:16, BSR d:8, and BSR d:16 (H8SX §2.4).
        first if first & 0xff0f == 0x5800 => {
            if word(bytes, 2)? & 1 == 0 {
                Some(4)
            } else {
                None
            }
        }
        first if first >> 8 == 0x55 && first & 1 == 0 => Some(2),
        0x5c00 => {
            if word(bytes, 2)? & 1 == 0 {
                Some(4)
            } else {
                None
            }
        }
        // RTS/L and RTE/L save 1–4 consecutive ER registers. The low
        // three bits encode the last register, so a group of n+1 cannot
        // end before ERn (H8SX §2.4, page 813).
        first if matches!(first >> 8, 0x54 | 0x56) && first & 0x00c0 == 0 => {
            let low = first & 0x003f;
            let count_minus_one = low >> 4;
            let last = low & 7;
            if low & 8 == 0 && last >= count_minus_one {
                Some(2)
            } else {
                None
            }
        }
        // JMP/JSR @aa:24: the first word carries eight address bits and
        // the following word carries sixteen (H8SX §2.4, page 730).
        first if matches!(first >> 8, 0x5a | 0x5e) => Some(4),
        // JMP/JSR @@aa:8: the low byte names a vector-table address.
        first if matches!(first >> 8, 0x5b | 0x5f) => Some(2),
        // JMP/JSR @ERn, @aa:32 and @@vec:7; BRA/BSR through an index
        // register share the 59/5d groups (H8SX §2.4, pages 696/730).
        first if matches!(first >> 8, 0x59 | 0x5d) => match first & 0x00ff {
            low if low & 0x80 != 0 => Some(2),             // @@vec:7
            low if low & 0x8f == 0 => Some(2),             // @ERn
            low if matches!(low & 0x8f, 5..=7) => Some(2), // PC-indexed branch
            0x08 => Some(6),                               // @aa:32
            _ => None,
        },
        // MOV/ADD/CMP/SUB/OR/XOR/AND immediate to register. The 79xx
        // word forms have a four-bit register field and one 16-bit
        // immediate word. In 7axx, bit 3 selects a 16-bit (set) or
        // 32-bit (clear) immediate, followed by a three-bit ER field.
        first if first >> 8 == 0x79 && (first & 0x00f0) < 0x70 => Some(4),
        first if first >> 8 == 0x7a && (first & 0x00f0) < 0x70 => {
            Some(if first & 8 == 0 { 6 } else { 4 })
        }
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
