use h8_asm::{
    isa::{decode_insn, disassemble_insn, encode_insn, CodecError, Ea, Insn, Operand, Reg, Size},
    Mode, Target,
};

#[test]
fn every_legacy_first_word_round_trips_its_semantic_decode() {
    for target in [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
    ] {
        let mode = if target == Target::H8_300 {
            Mode::Normal
        } else {
            Mode::Advanced
        };
        let mut counts = [0u32; 8];
        for first in 0..=u16::MAX {
            let mut bytes = [0u8; 14];
            bytes[..2].copy_from_slice(&first.to_be_bytes());
            if let Ok(decoded) = decode_insn(&bytes, target, mode) {
                assert!(!disassemble_insn(decoded.insn, target, mode)
                    .unwrap()
                    .is_empty());
                let encoded = encode_insn(decoded.insn, target, mode).unwrap_or_else(|error| {
                    panic!("{target:?} {first:04x}: {error:?}: {decoded:?}")
                });
                assert_eq!(
                    encoded,
                    &bytes[..decoded.len],
                    "{target:?} {first:04x}: {decoded:?}"
                );
                counts[decoded.len / 2] += 1;
            }
        }
        assert_eq!(
            counts,
            match target {
                Target::H8_300 => [0, 51380, 474, 0, 0, 0, 0, 0],
                Target::H8_300H => [0, 54344, 1249, 120, 0, 0, 0, 0],
                Target::H8S2000 => [0, 54696, 1249, 120, 0, 0, 0, 0],
                Target::H8S2600 => [0, 54729, 1249, 120, 0, 0, 0, 0],
                _ => unreachable!(),
            }
        );
    }
}

#[test]
fn meaningful_operands_and_fixed_bit_refusals() {
    let decoded = decode_insn(&[0x08, 0x18], Target::H8_300, Mode::Normal).unwrap();
    assert_eq!(
        decoded.insn,
        Insn::new(
            "ADD",
            Some(Size::Byte),
            [
                Operand::Register(Reg::Byte(1)),
                Operand::Register(Reg::Byte(8)),
                Operand::None
            ]
        )
    );
    assert_eq!(
        decode_insn(&[0x67, 0xb8], Target::H8_300, Mode::Normal)
            .unwrap()
            .insn,
        Insn::new(
            "BIST",
            None,
            [
                Operand::Immediate { value: 3, bits: 3 },
                Operand::Register(Reg::Byte(8)),
                Operand::None
            ]
        )
    );
    assert_eq!(
        decode_insn(&[], Target::H8_300, Mode::Normal),
        Err(CodecError::UnsupportedEncoding)
    );
    assert_eq!(
        decode_insn(&[0, 0], Target::H8_300, Mode::Maximum),
        Err(CodecError::UnsupportedTarget)
    );
    assert_eq!(
        decode_insn(&[0, 0], Target::H8SX, Mode::Maximum)
            .unwrap()
            .len,
        2
    );
    let invalid = Insn::new(
        "MOV",
        Some(Size::Word),
        [
            Operand::Immediate {
                value: 0x10000,
                bits: 16,
            },
            Operand::Register(Reg::Word(0)),
            Operand::None,
        ],
    );
    assert_eq!(
        encode_insn(invalid, Target::H8_300, Mode::Normal),
        Err(CodecError::UnsupportedOperands)
    );
    let invalid = Insn::new(
        "BRA",
        None,
        [
            Operand::Address(Ea::PcRelative {
                value: 128,
                bits: 8,
            }),
            Operand::None,
            Operand::None,
        ],
    );
    assert_eq!(
        encode_insn(invalid, Target::H8_300, Mode::Normal),
        Err(CodecError::UnsupportedOperands)
    );
}

fn checked(bytes: &[u8], target: Target) {
    let mode = if target == Target::H8_300 {
        Mode::Normal
    } else {
        Mode::Advanced
    };
    let len = h8_asm::isa::insn_len(bytes, target, mode).unwrap();
    let decoded = decode_insn(bytes, target, mode)
        .unwrap_or_else(|error| panic!("{target:?} {bytes:02x?}: {error:?}"));
    assert_eq!(decoded.len, len);
    assert!(!disassemble_insn(decoded.insn, target, mode)
        .unwrap()
        .is_empty());
    assert_eq!(
        encode_insn(decoded.insn, target, mode)
            .unwrap_or_else(|error| panic!("{bytes:02x?}: {decoded:?}: {error:?}")),
        bytes[..len]
    );
    for end in 0..len {
        assert!(decode_insn(&bytes[..end], target, mode).is_err());
    }
}

#[test]
fn every_legacy_prefix_second_word_has_semantics_and_round_trips() {
    for (target, expected) in [Target::H8_300H, Target::H8S2000, Target::H8S2600]
        .into_iter()
        .zip([1910, 3006, 3070])
    {
        let mut count = 0u32;
        for first in [
            0x0100u16, 0x0110, 0x0120, 0x0130, 0x0140, 0x0141, 0x0160, 0x01c0, 0x01d0, 0x01e0,
            0x01f0, 0x7800, 0x7870, 0x7b5c, 0x7bd4, 0x7c00, 0x7d70, 0x7e12, 0x7fff,
        ] {
            for second in 0..=u16::MAX {
                let mut bytes = [0u8; 14];
                bytes[..2].copy_from_slice(&first.to_be_bytes());
                bytes[2..4].copy_from_slice(&second.to_be_bytes());
                if h8_asm::isa::insn_len(&bytes, target, Mode::Advanced).is_some() {
                    checked(&bytes, target);
                    count += 1;
                }
            }
        }
        assert_eq!(count, expected, "{target:?} prefixed census");
    }
}

#[test]
fn wide_memory_extensions_and_bit_absolute_forms() {
    for target in [Target::H8_300H, Target::H8S2000, Target::H8S2600] {
        for prefix in [None, Some(0x0100u16), Some(0x0140), Some(0x0141)] {
            for base in 0..8u8 {
                for data in 0..16u8 {
                    for opcode in [0x6au8, 0x6b] {
                        for direction in [0u8, 0x80] {
                            for value in [0u32, 0x7fffff, 0x800000, 0xffffff, 0xffffffff] {
                                let mut bytes = Vec::new();
                                if let Some(prefix) = prefix {
                                    bytes.extend_from_slice(&prefix.to_be_bytes());
                                }
                                bytes.extend_from_slice(&[
                                    0x78,
                                    base << 4,
                                    opcode,
                                    0x20 | direction | data,
                                ]);
                                bytes.extend_from_slice(&value.to_be_bytes());
                                if h8_asm::isa::insn_len(&bytes, target, Mode::Advanced).is_some() {
                                    checked(&bytes, target);
                                }
                                // Preserve the documented long-store fourth-byte alias.
                                if prefix == Some(0x0100) {
                                    bytes[3] |= 0x80;
                                    if h8_asm::isa::insn_len(&bytes, target, Mode::Advanced)
                                        .is_some()
                                    {
                                        checked(&bytes, target);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for first in [0x6a10u16, 0x6a18, 0x6a30, 0x6a38] {
            for last in 0..=u16::MAX {
                let mut bytes = first.to_be_bytes().to_vec();
                if first & 0x20 == 0 {
                    bytes.extend_from_slice(&[0x80, 0x01]);
                } else {
                    bytes.extend_from_slice(&[0x80, 0, 0, 1]);
                }
                bytes.extend_from_slice(&last.to_be_bytes());
                if h8_asm::isa::insn_len(&bytes, target, Mode::Advanced).is_some() {
                    checked(&bytes, target);
                }
            }
        }
    }
}

#[test]
fn odd_long_relative_displacements_are_rejected_on_each_legacy_core() {
    for target in [Target::H8_300H, Target::H8S2000, Target::H8S2600] {
        for bytes in [[0x58, 0, 0, 1], [0x5c, 0, 0xff, 0xff]] {
            assert_eq!(h8_asm::isa::insn_len(&bytes, target, Mode::Advanced), None);
            assert_eq!(
                decode_insn(&bytes, target, Mode::Advanced),
                Err(CodecError::UnsupportedEncoding)
            );
        }
    }
}

#[test]
fn h8s_multiple_register_groups_are_not_h8sx_serial_ranges() {
    for target in [Target::H8S2000, Target::H8S2600] {
        for count in 1..=3u8 {
            for first in 0..8u8 {
                let valid = first + count < 8 && first % if count == 1 { 2 } else { 4 } == 0;
                let store = [1, count << 4, 0x6d, 0xf0 | first];
                assert_eq!(
                    h8_asm::isa::insn_len(&store, target, Mode::Normal),
                    if valid { Some(4) } else { None }
                );
                if first + count < 8 {
                    let load = [1, count << 4, 0x6d, 0x70 | (first + count)];
                    assert_eq!(
                        h8_asm::isa::insn_len(&load, target, Mode::Normal),
                        if valid { Some(4) } else { None }
                    );
                }
            }
        }
    }
    assert_eq!(
        h8_asm::isa::insn_len(&[1, 0x10, 0x6d, 0x72], Target::H8SX, Mode::Normal),
        Some(4)
    );
}

#[test]
fn manual_context_preserves_absolute_widths_and_control_operation_sizes() {
    // REJ09B0213/REJ09B0139 instruction-code tables: the same absolute
    // extension is aa:24 on H8/300H and aa:32 on H8S.
    for (target, bits) in [(Target::H8_300H, 24), (Target::H8S2000, 32)] {
        let decoded =
            decode_insn(&[0x6a, 0x20, 0, 0x12, 0x34, 0x56], target, Mode::Advanced).unwrap();
        assert_eq!(
            decoded.insn.operands[0],
            Operand::Address(Ea::Absolute {
                value: 0x123456,
                bits
            })
        );
    }
    for (target, mode, bits) in [
        (Target::H8_300, Mode::Normal, 16),
        (Target::H8_300H, Mode::Advanced, 24),
    ] {
        for opcode in [0x5a, 0x5e] {
            assert_eq!(
                decode_insn(&[opcode, 0, 0x12, 0x34], target, mode)
                    .unwrap()
                    .insn
                    .operands[0],
                Operand::Address(Ea::Absolute {
                    value: 0x1234,
                    bits
                })
            );
        }
    }
    for opcode in 4..=7u8 {
        let expected = if opcode == 7 { Some(Size::Byte) } else { None };
        assert_eq!(
            decode_insn(&[opcode, 1], Target::H8_300, Mode::Normal)
                .unwrap()
                .insn
                .size,
            expected
        );
        assert_eq!(
            decode_insn(&[1, 0x41, opcode, 1], Target::H8S2000, Mode::Advanced)
                .unwrap()
                .insn
                .size,
            expected
        );
    }
    assert_eq!(
        decode_insn(&[0x0b, 0], Target::H8_300, Mode::Normal)
            .unwrap()
            .insn
            .size,
        Some(Size::Word)
    );
    assert_eq!(
        decode_insn(&[0x0b, 0], Target::H8_300H, Mode::Advanced)
            .unwrap()
            .insn
            .size,
        Some(Size::Long)
    );
}
