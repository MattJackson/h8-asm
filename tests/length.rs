use h8_asm::{
    isa::{insn_len, MAX_INSN_LEN},
    Mode, Target,
};

const TARGETS: [Target; 5] = [
    Target::H8_300,
    Target::H8_300H,
    Target::H8S2000,
    Target::H8S2600,
    Target::H8SX,
];
const MODES: [Mode; 4] = [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum];

// A zero-filled suffix census is reproducible, but is not an allocation
// census: prefixes requiring a nonzero second opcode are rejected here.
#[test]
fn first_word_zero_suffix_census() {
    for (target, expected) in TARGETS.into_iter().zip([
        [13682, 51380, 474, 0, 0, 0, 0, 0],
        [9823, 54344, 1249, 120, 0, 0, 0, 0],
        [9471, 54696, 1249, 120, 0, 0, 0, 0],
        [9438, 54729, 1249, 120, 0, 0, 0, 0],
        [7859, 56080, 1392, 204, 1, 0, 0, 0],
    ]) {
        let mut counts = [0usize; 8];
        for first in 0..=u16::MAX {
            let mut bytes = [0; 14];
            bytes[..2].copy_from_slice(&first.to_be_bytes());
            let len = insn_len(&bytes, target, Mode::Normal);
            counts[len.unwrap_or(0) / 2] += 1;
            if let Some(len) = len {
                for end in 0..len {
                    assert_eq!(insn_len(&bytes[..end], target, Mode::Normal), None);
                }
                assert_eq!(insn_len(&bytes[..len], target, Mode::Normal), Some(len));
            }
            for mode in MODES {
                assert_eq!(
                    insn_len(&bytes, target, mode),
                    if target.supports(mode) { len } else { None }
                );
            }
        }
        assert_eq!(counts, expected, "{target:?}");
    }
}

#[test]
fn h8sx_byte_immediate_register_space() {
    // H8SX §2.4 rows ADD/ADDX/CMP/SUBX/OR/XOR/AND/MOV .B #xx:8,Rd.
    // Each high nibble 8..f includes every byte register and immediate.
    for first in 0x8000u16..=0xffff {
        let bytes = first.to_be_bytes();
        assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(2));
        assert_eq!(insn_len(&bytes[..1], Target::H8SX, Mode::Normal), None);
    }
}

#[test]
fn legacy_pc_relative_displacements_are_even() {
    // H8/300 §2 Bcc/BSR requires an even displacement; H8/300H and
    // H8S retain the even branch-destination requirement.
    for target in TARGETS.into_iter().filter(|target| *target != Target::H8SX) {
        for high in (0x40u8..=0x4f).chain([0x55]) {
            for low in 0..=u8::MAX {
                assert_eq!(
                    insn_len(&[high, low], target, Mode::Normal),
                    if low & 1 == 0 { Some(2) } else { None },
                    "{target:?} {high:02x}{low:02x}"
                );
            }
        }
    }
}

#[test]
fn h8sx_ccr_and_register_bit_operations() {
    // H8SX §2.4: ORC/XORC/ANDC/LDC.B #xx:8,CCR; register bit
    // operations BSET/BNOT/BCLR/BTST and BST/BIST.
    for high in [0x04u8, 0x05, 0x06, 0x07, 0x60, 0x61, 0x62, 0x63, 0x67] {
        for low in 0..=u8::MAX {
            assert_eq!(insn_len(&[high, low], Target::H8SX, Mode::Normal), Some(2));
        }
    }
    // H8SX §2.4: immediate bit operations to a byte register.
    for high in 0x70u8..=0x77 {
        for low in 0..=u8::MAX {
            assert_eq!(
                insn_len(&[high, low], Target::H8SX, Mode::Normal),
                if high <= 0x73 && low & 0x80 != 0 {
                    None
                } else {
                    Some(2)
                }
            );
        }
    }
}

#[test]
fn h8sx_byte_control_register_transfers() {
    // H8SX §2.4 LDC.B/STC.B direct CCR and EXR forms.
    for high in [0x02u8, 0x03] {
        for low in 0..=u8::MAX {
            assert_eq!(
                insn_len(&[high, low], Target::H8SX, Mode::Normal),
                if low < 0x20 || matches!(low & 0xf8, 0x20 | 0x30 | 0x60 | 0x70) {
                    Some(2)
                } else {
                    None
                }
            );
        }
    }
}

#[test]
fn h8sx_unsigned_multiply_divide_register_space() {
    // H8SX §2.4 MULXU/DIVXU .B and .W register forms.
    for high in 0x50u8..=0x53 {
        for low in 0..=u8::MAX {
            assert_eq!(
                insn_len(&[high, low], Target::H8SX, Mode::Normal),
                if high <= 0x51 || low & 8 == 0 {
                    Some(2)
                } else {
                    None
                }
            );
        }
    }
}

#[test]
fn h8sx_trap_vectors() {
    // H8SX §2.4 TRAPA #x:2 has four vectors and fixed reserved bits.
    for low in 0..=u8::MAX {
        assert_eq!(
            insn_len(&[0x57, low], Target::H8SX, Mode::Normal),
            if low & 0xcf == 0 { Some(2) } else { None }
        );
    }
}

#[test]
fn h8sx_adds_subs_and_increment_decrement() {
    // H8SX §2.4: three-bit ER fields for ADDS/SUBS and long INC/DEC;
    // four-bit word-register fields for word INC/DEC.
    for high in [0x0bu8, 0x1b] {
        for low in 0..=u8::MAX {
            let expected = matches!(low & 0xf8, 0x00 | 0x80 | 0x90 | 0x70 | 0xf0)
                || matches!(low & 0xf8, 0x50 | 0x58 | 0xd0 | 0xd8);
            assert_eq!(
                insn_len(&[high, low], Target::H8SX, Mode::Normal),
                if expected { Some(2) } else { None },
                "{high:02x}{low:02x}"
            );
        }
    }
}

#[test]
fn h8sx_shift_and_rotate_register_space() {
    // H8SX §2.4 SHLL/SHLR/SHAL/SHAR and ROTXL/ROTXR/ROTL/ROTR
    // register rows, including the listed fixed shift counts.
    for high in 0x10u8..=0x13 {
        for low in 0..=u8::MAX {
            let group = low >> 4;
            let expected = if high <= 0x11 {
                matches!(group, 0x0..=0xa | 0xc | 0xd | 0xf) || group == 0xb && low & 8 == 0
            } else {
                matches!(group, 0 | 1 | 4 | 5 | 8 | 9 | 12 | 13)
                    || matches!(group, 3 | 7 | 11 | 15) && low & 8 == 0
            };
            assert_eq!(
                insn_len(&[high, low], Target::H8SX, Mode::Normal),
                if expected { Some(2) } else { None },
                "{high:02x}{low:02x}"
            );
        }
    }
}

#[test]
fn h8sx_unary_register_space() {
    // H8SX §2.4 NOT/NEG/EXTU/EXTS register rows. The long rows use
    // a three-bit ER register and fixed-zero bit 3.
    for low in 0..=u8::MAX {
        let group = low >> 4;
        let expected = matches!(group, 0 | 1 | 5 | 8 | 9 | 13)
            || matches!(group, 3 | 6 | 7 | 11 | 14 | 15) && low & 8 == 0;
        assert_eq!(
            insn_len(&[0x17, low], Target::H8SX, Mode::Normal),
            if expected { Some(2) } else { None },
            "17{low:02x}"
        );
    }
}

#[test]
fn h8sx_byte_register_and_absolute_space() {
    // H8SX §2.4: ADD.B Rs,Rd (08), MOV.B Rs,Rd (0c), and
    // MOV.B to/from @aa:8 (20..3f). All low-byte fields are allocated.
    for high in [0x08u8, 0x0c].into_iter().chain(0x20..=0x3f) {
        for low in 0..=u8::MAX {
            let bytes = [high, low];
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(2));
        }
    }
}

#[test]
fn h8sx_word_and_long_immediate_register_space() {
    // H8SX §2.4, seven operations each for 79xx (.W) and 7axx (.L).
    for operation in 0..=6u8 {
        for register in 0..=15u8 {
            let word = [0x79, (operation << 4) | register, 0x12, 0x34];
            for end in 0..word.len() {
                assert_eq!(insn_len(&word[..end], Target::H8SX, Mode::Maximum), None);
            }
            assert_eq!(insn_len(&word, Target::H8SX, Mode::Maximum), Some(4));

            let long = [0x7a, (operation << 4) | register, 0x12, 0x34, 0x56, 0x78];
            assert_eq!(
                insn_len(&long, Target::H8SX, Mode::Maximum),
                Some(if register < 8 { 6 } else { 4 })
            );
            if register < 8 {
                for end in 0..long.len() {
                    assert_eq!(insn_len(&long[..end], Target::H8SX, Mode::Maximum), None);
                }
            } else {
                for end in 0..4 {
                    assert_eq!(insn_len(&long[..end], Target::H8SX, Mode::Maximum), None);
                }
            }
        }
    }
    for high in [0x79u8, 0x7a] {
        for low in 0x70..=0xff {
            assert_eq!(
                insn_len(&[high, low, 0, 0, 0, 0], Target::H8SX, Mode::Normal),
                // §2.4: immediate-to-memory MOV and MOVA index rows.
                if (high == 0x79 && low == 0x74) || (high == 0x7a && low == 0x7c) {
                    Some(6)
                } else if high == 0x7a && (0x80..=0xdf).contains(&low) {
                    Some(if low & 8 == 0 { 6 } else { 4 })
                } else {
                    None
                }
            );
        }
    }
}

#[test]
fn h8sx_jump_call_and_index_branch_patterns() {
    // H8SX §2.4 Table 2.2, manual pages 696, 730. For 59/5d the low
    // byte selects register-indirect, PC-indexed, full absolute, or vector.
    for high in [0x59u8, 0x5d] {
        for low in 0..=u8::MAX {
            let bytes = [high, low, 0x12, 0x34, 0x56, 0x78];
            let expected = if low & 0x80 != 0 || low & 0x8f == 0 || (5..=7).contains(&(low & 0x8f))
            {
                Some(2)
            } else if low == 8 {
                Some(6)
            } else {
                None
            };
            assert_eq!(
                insn_len(&bytes, Target::H8SX, Mode::Maximum),
                expected,
                "{high:02x}{low:02x}"
            );
            if let Some(len) = expected {
                for end in 0..len {
                    assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Maximum), None);
                }
            }
        }
    }
    for high in [0x5au8, 0x5e] {
        for low in 0..=u8::MAX {
            let bytes = [high, low, 0x12, 0x34];
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
            assert_eq!(insn_len(&bytes[..3], Target::H8SX, Mode::Normal), None);
        }
    }
    for high in [0x5bu8, 0x5f] {
        for low in 0..=u8::MAX {
            assert_eq!(insn_len(&[high, low], Target::H8SX, Mode::Normal), Some(2));
        }
    }
}

#[test]
fn h8sx_long_return_register_ranges() {
    // H8SX §2.4 Table 2.2, manual page 813. The low three bits
    // encode the final ER register in a consecutive group of 1–4.
    for high in [0x54u8, 0x56] {
        for count_minus_one in 0..=3u8 {
            for last in 0..=7u8 {
                let low = (count_minus_one << 4) | last;
                assert_eq!(
                    insn_len(&[high, low], Target::H8SX, Mode::Maximum),
                    if last >= count_minus_one {
                        Some(2)
                    } else {
                        None
                    },
                    "{high:02x}{low:02x}"
                );
                assert_eq!(
                    insn_len(&[high, low | 8], Target::H8SX, Mode::Maximum),
                    None
                );
            }
        }
        assert_eq!(insn_len(&[high, 0x70], Target::H8SX, Mode::Normal), Some(2));
    }
}

#[test]
fn h8sx_word_and_long_register_pairs() {
    // H8SX §2.4 Table 2.2, arithmetic, logic, compare and move rows.
    for high in [0x09u8, 0x0d, 0x19, 0x1d, 0x64, 0x65, 0x66] {
        for low in 0..=u8::MAX {
            assert_eq!(insn_len(&[high, low], Target::H8SX, Mode::Normal), Some(2));
        }
    }
    for high in [0x0au8, 0x0f, 0x1a, 0x1f] {
        for low in 0..=u8::MAX {
            assert_eq!(
                insn_len(&[high, low], Target::H8SX, Mode::Normal),
                if low & 0x80 == 0 || low & 0x88 == 0x80 || low & 0x88 == 0x88 && low & 0x70 != 0 {
                    Some(2)
                } else {
                    None
                },
                "{high:02x}{low:02x}"
            );
        }
    }
}

#[test]
fn h8sx_remaining_byte_register_pairs() {
    // H8SX §2.4: ADDX, OR, XOR, AND, SUB, CMP and SUBX .B Rs,Rd.
    for high in [0x0eu8, 0x14, 0x15, 0x16, 0x18, 0x1c, 0x1e] {
        for low in 0..=u8::MAX {
            assert_eq!(insn_len(&[high, low], Target::H8SX, Mode::Normal), Some(2));
        }
    }
}

#[test]
fn h8sx_register_indirect_and_displacement_moves() {
    // H8SX §2.4 MOV.B/W rows on manual pages 751–760.
    for high in [0x68u8, 0x69, 0x6c, 0x6d] {
        for low in 0..=u8::MAX {
            assert_eq!(insn_len(&[high, low], Target::H8SX, Mode::Normal), Some(2));
        }
    }
    for high in [0x6eu8, 0x6f] {
        for low in 0..=u8::MAX {
            let bytes = [high, low, 0x12, 0x34];
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
            for end in 0..4 {
                assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
            }
        }
    }
}

#[test]
fn h8sx_absolute_byte_and_word_moves() {
    // H8SX §2.4 MOV.B/W @aa:16/@aa:32 rows, manual pages 752–760.
    for high in [0x6au8, 0x6b] {
        for field in [0x00u8, 0x20, 0x80, 0xa0] {
            let len = if field & 0x20 == 0 { 4 } else { 6 };
            for reg in 0..=15u8 {
                let bytes = [high, field | reg, 0x12, 0x34, 0x56, 0x78];
                assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Maximum), Some(len));
                for end in 0..len {
                    assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Maximum), None);
                }
            }
        }
        for field in [
            0x10u8, 0x30, 0x40, 0x50, 0x60, 0x70, 0x90, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0,
        ] {
            assert_eq!(
                insn_len(&[high, field, 0, 0, 0, 0], Target::H8SX, Mode::Maximum),
                // MOVFPE/TPE and short-immediate absolute MOV rows.
                if field == 0xd0 || high == 0x6a && matches!(field, 0x40 | 0xc0) {
                    Some(4)
                } else if field == 0xf0 {
                    Some(6)
                } else {
                    None
                }
            );
        }
    }
}

#[test]
fn h8sx_prefixed_long_moves() {
    // H8SX §2.4 MOV.L single-memory-operand rows, manual page 752.
    for second_high in [0x69u8, 0x6d, 0x6f] {
        for second_low in 0..=u8::MAX {
            let bytes = [0x01, 0x00, second_high, second_low, 0x12, 0x34];
            let expected = if second_low & 8 == 0 {
                Some(if second_high == 0x6f { 6 } else { 4 })
            } else {
                None
            };
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), expected);
            if let Some(len) = expected {
                for end in 0..len {
                    assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
                }
            }
        }
    }
    for second_low in 0..=u8::MAX {
        let bytes = [0x01, 0x00, 0x6b, second_low, 0x12, 0x34, 0x56, 0x78];
        let expected = match second_low & 0xf8 {
            0x00 | 0x80 => Some(6),
            0x20 | 0xa0 => Some(8),
            _ => None,
        };
        assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Maximum), expected);
        if let Some(len) = expected {
            for end in 0..len {
                assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Maximum), None);
            }
        }
    }
    assert_eq!(
        insn_len(&[0x01, 0x00, 0x00, 0x00], Target::H8SX, Mode::Normal),
        None
    );
}

// H8SX REJ09B0102 §2.4, operation-only rows (NOP, SLEEP, RTS, RTE).
#[test]
fn h8sx_fixed_opcodes_and_refusal() {
    for (bytes, name) in [
        ([0x00, 0x00], "NOP"),
        ([0x01, 0x80], "SLEEP"),
        ([0x54, 0x70], "RTS"),
        ([0x56, 0x70], "RTE"),
    ] {
        for mode in MODES {
            assert_eq!(insn_len(&bytes, Target::H8SX, mode), Some(2), "{name}");
            assert_eq!(insn_len(&bytes[..1], Target::H8SX, mode), None, "{name}");
            assert_eq!(
                insn_len(&[bytes[0], bytes[1], 0xff], Target::H8SX, mode),
                Some(2),
                "{name}"
            );
        }
    }
    // An allocated extended EA row starts with 78xx, but the remaining
    // opcode/EA fields are not yet recognized for H8SX.
    assert_eq!(
        insn_len(&[0x78, 0x04, 0x6a, 0x2c], Target::H8SX, Mode::Maximum),
        None
    );
}

#[test]
fn h8sx_branch_lengths_and_reserved_bits() {
    // H8SX §2.4 Bcc, BRA/S, BSR. BRA/S has a delay slot (§2.2.24),
    // which the future relocator must treat as a pair with this instruction.
    for condition in 0..=15u8 {
        for displacement in [0x00, 0x7e, 0x80, 0xfe] {
            let bytes = [0x40 | condition, displacement];
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(2));
        }
        let bytes = [0x58, condition << 4, 0x12, 0x34];
        assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Maximum), Some(4));
        assert_eq!(insn_len(&bytes[..3], Target::H8SX, Mode::Maximum), None);
        let reserved = [0x58, (condition << 4) | 1, 0x12, 0x34];
        assert_eq!(insn_len(&reserved, Target::H8SX, Mode::Normal), None);
    }
    for bytes in [[0x40, 0x01], [0x40, 0x7f], [0x40, 0xff]] {
        assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(2));
    }
    for condition in 1..=15u8 {
        assert_eq!(
            insn_len(&[0x40 | condition, 1], Target::H8SX, Mode::Normal),
            None
        );
    }
    for displacement in [0, 1, 0x7f, 0xff] {
        assert_eq!(
            insn_len(&[0x55, displacement], Target::H8SX, Mode::Normal),
            if displacement & 1 == 0 { Some(2) } else { None }
        );
    }
    assert_eq!(
        insn_len(&[0x58, 0x00, 0x00, 0x01], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x5c, 0x00, 0x00, 0x01], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x5c, 0x00, 0xff, 0xfe], Target::H8SX, Mode::Normal),
        Some(4)
    );
    assert_eq!(
        insn_len(&[0x5c, 0x01, 0xff, 0xfe], Target::H8SX, Mode::Normal),
        None
    );
}

// H8SX REJ09B0102 §2.4 Table 2.2, ADD.B
// @(d:32,ERs),@(d:32,ERd), manual page 640 / PDF page 658.
#[test]
fn h8sx_two_32_bit_displacements() {
    assert_eq!(MAX_INSN_LEN, 14);
    for source in 0..8 {
        for destination in 0..8 {
            let bytes = [
                0x78,
                (source << 4) | 0x04,
                0x6a,
                0x2c,
                0x12,
                0x34,
                0x56,
                0x78,
                0xc8 | destination,
                0x10,
                0x9a,
                0xbc,
                0xde,
                0xf0,
            ];
            for end in 0..bytes.len() {
                assert_eq!(
                    insn_len(&bytes[..end], Target::H8SX, Mode::Maximum),
                    None,
                    "source {source}, destination {destination}, end {end}"
                );
            }
            for mode in MODES {
                assert_eq!(
                    insn_len(&bytes, Target::H8SX, mode),
                    Some(14),
                    "source {source}, destination {destination}, {mode:?}"
                );
                let mut followed = bytes.to_vec();
                followed.extend_from_slice(&[0xff, 0xff]);
                assert_eq!(insn_len(&followed, Target::H8SX, mode), Some(14));
            }
        }
    }

    let mut bytes = [
        0x78, 0x04, 0x6a, 0x2c, 0x12, 0x34, 0x56, 0x78, 0xc8, 0x10, 0x9a, 0xbc, 0xde, 0xf0,
    ];
    for (at, replacement) in [(1, 0x05), (2, 0x6b), (3, 0x2d), (8, 0x58), (9, 0x11)] {
        let old = bytes[at];
        bytes[at] = replacement;
        assert_eq!(
            insn_len(&bytes, Target::H8SX, Mode::Normal),
            // Source byte indexing and word-size operations are now supported.
            if at == 1 || at == 2 { Some(14) } else { None }
        );
        bytes[at] = old;
    }
}

#[test]
fn h8sx_add_byte_destination_ea_lengths() {
    // H8SX §2.4 Table 2.2, manual page 640. Third opcode word encodes
    // destination mode; 0/16/32-bit EA extensions follow it.
    for (third, len) in [
        (0x0010u16, 10), // @ER0
        (0x8010, 10),    // @ER0+
        (0x9010, 10),    // @+ER0
        (0xa010, 10),    // @ER0-
        (0xb010, 10),    // @-ER0
        (0xc010, 12),    // @(d:16,ER0)
        (0xd010, 12),    // @(d:16,R0L.B)
        (0xe010, 12),    // @(d:16,R0.W)
        (0xf010, 12),    // @(d:16,ER0.L)
        (0xc810, 14),    // @(d:32,ER0)
        (0xd810, 14),    // @(d:32,R0L.B)
        (0xe810, 14),    // @(d:32,R0.W)
        (0xf810, 14),    // @(d:32,ER0.L)
        (0x4010, 12),    // @aa:16
        (0x4810, 14),    // @aa:32
    ] {
        let mut bytes = [0u8; 14];
        bytes[..4].copy_from_slice(&[0x78, 0x04, 0x6a, 0x2c]);
        bytes[8..10].copy_from_slice(&third.to_be_bytes());
        for end in 0..len {
            assert_eq!(
                insn_len(&bytes[..end], Target::H8SX, Mode::Maximum),
                None,
                "{third:04x} / {end}"
            );
        }
        for mode in MODES {
            assert_eq!(
                insn_len(&bytes, Target::H8SX, mode),
                Some(len),
                "{third:04x} / {mode:?}"
            );
        }
        if third != 0x4010 && third != 0x4810 {
            for destination in 0..8 {
                bytes[8] = (third >> 8) as u8 | destination;
                assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Maximum), Some(len));
            }
        }
    }
    // The two absolute-address rows have a fixed destination field.
    for third in [0x4110u16, 0x4910] {
        let mut bytes = [0u8; 14];
        bytes[..4].copy_from_slice(&[0x78, 0x04, 0x6a, 0x2c]);
        bytes[8..10].copy_from_slice(&third.to_be_bytes());
        assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Maximum), None);
    }
}

// Manual witnesses: H8/300 Appendix B; H8/300H §2.4 Table 2.3;
// H8S §2.4 Table 2.2. These exercise opcode words beyond the first word.
#[test]
fn extension_and_prefix_boundaries() {
    let cases: &[(Target, &[u8])] = &[
        (Target::H8_300, &[0x7b, 0x5c, 0x59, 0x8f]),  // EEPMOV
        (Target::H8_300, &[0x7c, 0x70, 0x73, 0x70]),  // BTST
        (Target::H8_300H, &[0x01, 0x00, 0x69, 0x07]), // MOV.L
        (Target::H8_300H, &[0x01, 0x00, 0x6f, 0x07, 0xff, 0xff]),
        (
            Target::H8_300H,
            &[0x01, 0x00, 0x6b, 0x27, 0, 0xff, 0xff, 0xff],
        ),
        (
            Target::H8S2000,
            &[0x01, 0x00, 0x78, 0x70, 0x6b, 0x27, 0xff, 0xff, 0xff, 0xff],
        ),
        (Target::H8S2000, &[0x6a, 0x10, 0xff, 0xff, 0x73, 0x70]),
        (
            Target::H8S2000,
            &[0x6a, 0x38, 0xff, 0xff, 0xff, 0xff, 0x70, 0x70],
        ),
        (Target::H8S2000, &[0x01, 0x41, 0x04, 0xff]), // ORC EXR
        (Target::H8S2600, &[0x01, 0x60, 0x6d, 0x07]), // MAC
    ];
    for &(target, bytes) in cases {
        for end in 0..bytes.len() {
            assert_eq!(
                insn_len(&bytes[..end], target, Mode::Normal),
                None,
                "{bytes:02x?}"
            );
        }
        assert_eq!(
            insn_len(bytes, target, Mode::Normal),
            Some(bytes.len()),
            "{bytes:02x?}"
        );
        let mut padded = bytes.to_vec();
        padded.extend_from_slice(&[0xff; 4]);
        assert_eq!(insn_len(&padded, target, Mode::Normal), Some(bytes.len()));
    }
}

#[test]
fn reserved_bits_and_generation_restrictions() {
    // H8/300H's 24-bit absolute EA has a zero high byte; H8S uses 32 bits.
    let ea = [0x6a, 0x20, 1, 0, 0, 0];
    assert_eq!(insn_len(&ea, Target::H8_300H, Mode::Advanced), None);
    assert_eq!(insn_len(&ea, Target::H8S2000, Mode::Advanced), Some(6));
    // H8S Table 2.2 note 3: TAS permits only ER0/1/4/5.
    for reg in 0..8 {
        let bytes = [0x01, 0xe0, 0x7b, (reg << 4) | 0x0c];
        assert_eq!(
            insn_len(&bytes, Target::H8S2000, Mode::Normal),
            if [0, 1, 4, 5].contains(&reg) {
                Some(4)
            } else {
                None
            }
        );
        assert_eq!(insn_len(&bytes, Target::H8_300H, Mode::Normal), None);
    }
    for bytes in [
        &[0x7b, 0x5c, 0x59, 0x8e][..],
        &[0x7c, 0x00, 0x70, 0x00],
        &[0x7d, 0x00, 0x73, 0x00],
    ] {
        assert_eq!(insn_len(bytes, Target::H8_300, Mode::Normal), None);
    }
    assert_eq!(insn_len(&[0x01, 0xa0], Target::H8S2000, Mode::Normal), None);
    assert_eq!(
        insn_len(&[0x01, 0xa0], Target::H8S2600, Mode::Normal),
        Some(2)
    );
}

#[test]
fn prefixed_move_and_register_group_fields() {
    // H8S Table 2.2: LDC/STC displacement and absolute forms, MOV.L
    // store alias (note 2), and LDM/STM register groups.
    for (bytes, expected) in [
        ([0x01, 0x40, 0x78, 0x00, 0x6b, 0x20, 0, 0, 0, 0], Some(10)),
        ([0x01, 0x00, 0x78, 0x80, 0x6b, 0xa0, 0, 0, 0, 0], Some(10)),
        ([0x01, 0x00, 0x6b, 0x00, 0, 0, 0, 0, 0, 0], Some(6)),
        ([0x01, 0x00, 0x6b, 0x40, 0, 0, 0, 0, 0, 0], None),
        ([0x01, 0x00, 0x69, 0x08, 0, 0, 0, 0, 0, 0], None),
    ] {
        assert_eq!(insn_len(&bytes, Target::H8S2000, Mode::Normal), expected);
    }
    for count in 1..=3 {
        for reg in 0..8 {
            let load = [0x01, count << 4, 0x6d, 0x70 | reg];
            let store = [0x01, count << 4, 0x6d, 0xf0 | reg];
            assert_eq!(
                insn_len(&load, Target::H8S2000, Mode::Normal),
                if reg >= count && (reg - count) % (if count == 1 { 2 } else { 4 }) == 0 {
                    Some(4)
                } else {
                    None
                }
            );
            assert_eq!(
                insn_len(&store, Target::H8S2000, Mode::Normal),
                if reg + count <= 7 && reg % (if count == 1 { 2 } else { 4 }) == 0 {
                    Some(4)
                } else {
                    None
                }
            );
        }
    }
}

#[test]
fn legacy_register_widths_and_memory_bit_prefixes_follow_manual_masks() {
    for opcode in [0x50, 0x51, 0x69, 0x6d] {
        assert_eq!(
            insn_len(&[opcode, 0], Target::H8_300, Mode::Normal),
            Some(2)
        );
        assert_eq!(insn_len(&[opcode, 8], Target::H8_300, Mode::Normal), None);
        assert_eq!(
            insn_len(&[opcode, 8], Target::H8_300H, Mode::Advanced),
            Some(2)
        );
    }
    for target in [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
    ] {
        for low in 0..=255u8 {
            let eepmov = [0x7b, low, 0x59, 0x8f];
            let valid = low == 0x5c || (low == 0xd4 && target != Target::H8_300);
            assert_eq!(
                insn_len(&eepmov, target, Mode::Normal),
                if valid { Some(4) } else { None }
            );
            assert_eq!(
                insn_len(&[0x7c, low, 0x73, 0], target, Mode::Normal),
                if [0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70].contains(&low) {
                    Some(4)
                } else {
                    None
                }
            );
            assert_eq!(
                insn_len(&[0x7d, low, 0x70, 0], target, Mode::Normal),
                if [0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70].contains(&low) {
                    Some(4)
                } else {
                    None
                }
            );
            assert_eq!(
                insn_len(&[0x7e, low, 0x73, 0], target, Mode::Normal),
                Some(4)
            );
            assert_eq!(
                insn_len(&[0x7f, low, 0x70, 0], target, Mode::Normal),
                Some(4)
            );
        }
    }
    let extended = [0x78, 0, 0x6a, 0x20, 0, 0, 0, 0];
    assert_eq!(insn_len(&extended, Target::H8_300, Mode::Normal), None);
    assert_eq!(insn_len(&extended, Target::H8_300H, Mode::Normal), Some(8));
}

#[test]
fn extended_absolute_bit_operations_require_h8s_and_reserved_zero_fields() {
    let bit = [0x6a, 0x10, 0, 0, 0x73, 0];
    for target in [Target::H8_300, Target::H8_300H] {
        assert_eq!(insn_len(&bit, target, Mode::Normal), None);
    }
    for target in [Target::H8S2000, Target::H8S2600] {
        assert_eq!(insn_len(&bit, target, Mode::Normal), Some(6));
        for prefix in [[0x6a, 0x11], [0x6a, 0x12], [0x6a, 0x14], [0x6b, 0x10]] {
            let mut invalid = bit;
            invalid[..2].copy_from_slice(&prefix);
            assert_eq!(insn_len(&invalid, target, Mode::Normal), None);
        }
    }
    assert_eq!(
        insn_len(&[0x6f, 0, 0, 0], Target::H8_300, Mode::Normal),
        Some(4)
    );
    assert_eq!(
        insn_len(&[0x6f, 8, 0, 0], Target::H8_300, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x6f, 8, 0, 0], Target::H8_300H, Mode::Normal),
        Some(4)
    );
}
