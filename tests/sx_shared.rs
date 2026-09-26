use h8_asm::{
    isa::{decode_insn, disassemble_insn, encode_insn, insn_len},
    Mode, Target,
};

pub const ROWS: &[u8] = include_bytes!("data/sx_rows.bin");

#[test]
fn every_manual_row_round_trips_typed_fields() {
    assert_eq!(ROWS.len(), 8493 * 71);
    for (index, record) in ROWS.chunks_exact(71).enumerate() {
        let len = usize::from(record[0]);
        for start in [1, 15, 29, 43, 57] {
            let bytes = &record[start..start + len];
            let decoded = decode_insn(bytes, Target::H8SX, Mode::Maximum).unwrap();
            assert_eq!(decoded.len, len);
            assert_eq!(
                encode_insn(decoded.insn, Target::H8SX, Mode::Maximum).unwrap(),
                bytes,
                "row {index}: {:?}",
                decoded.insn
            );
            assert!(!disassemble_insn(decoded.insn, Target::H8SX, Mode::Maximum)
                .unwrap()
                .is_empty());
        }
    }
}

#[test]
fn every_sx_two_byte_word_has_shared_semantics_or_is_refused() {
    let mut accepted = 0;
    for word in 0..=u16::MAX {
        let bytes = word.to_be_bytes();
        let decoded = decode_insn(&bytes, Target::H8SX, Mode::Maximum);
        assert_eq!(
            decoded.is_ok(),
            insn_len(&bytes, Target::H8SX, Mode::Maximum).is_some()
        );
        if let Ok(decoded) = decoded {
            assert_eq!(
                encode_insn(decoded.insn, Target::H8SX, Mode::Maximum).unwrap(),
                bytes
            );
            accepted += 1;
        }
    }
    assert_eq!(accepted, 56080);
}

#[test]
fn invalid_requests_and_changed_fields_cannot_be_silently_encoded() {
    use h8_asm::isa::{Ea, Encoding, Insn, Operand as O, Reg, Size};
    let target = Target::H8SX;
    let mode = Mode::Maximum;
    let nop = Insn::new("NOP", None, [O::None; 3]);
    for encoding in (0..8493).map(Encoding::SxAlternative).chain([
        Encoding::SxAlternative(u16::MAX),
        Encoding::DisplacementStoreAlias,
    ]) {
        let mut insn = nop;
        insn.encoding = encoding;
        assert!(encode_insn(insn, target, mode).is_err());
    }
    for insn in [
        Insn::new("UNKNOWN", None, [O::None; 3]),
        Insn::new("NOP", Some(Size::Byte), [O::None; 3]),
    ] {
        assert!(encode_insn(insn, target, mode).is_err());
    }
    let operands = [
        O::None,
        O::Register(Reg::Byte(255)),
        O::Register(Reg::Word(255)),
        O::Register(Reg::Long(255)),
        O::Register(Reg::Ccr),
        O::Immediate {
            value: u32::MAX,
            bits: 3,
        },
        O::Immediate {
            value: 0,
            bits: 255,
        },
        O::Address(Ea::Indexed {
            index: Reg::Byte(0),
            value: -1,
            bits: 16,
        }),
        O::Address(Ea::PostIncrement(Reg::Long(8))),
        O::Address(Ea::Displacement {
            base: Reg::Long(0),
            value: 3,
            bits: 2,
        }),
        O::Address(Ea::PcRelative { value: 1, bits: 8 }),
        O::Address(Ea::ExtendedIndirect(255)),
        O::Registers { first: 8, last: 0 },
        O::IndexedMemory {
            address: Ea::Indirect(Reg::Long(0)),
            index_size: Size::Long,
            value: 0,
            bits: 16,
        },
    ];
    for record in ROWS.chunks_exact(71) {
        let original = decode_insn(&record[1..1 + usize::from(record[0])], target, mode)
            .unwrap()
            .insn;
        for slot in 0..3 {
            for operand in operands {
                let mut insn = original;
                insn.operands[slot] = operand;
                if let Ok(bytes) = encode_insn(insn, target, mode) {
                    assert_eq!(decode_insn(&bytes, target, mode).unwrap().insn, insn);
                }
            }
        }
    }
}

#[test]
fn mova_preserves_high_byte_and_extended_word_register_identity() {
    use h8_asm::isa::{Ea, Operand, Reg};
    for (bytes, register) in [
        ([0x78, 8, 0x7a, 0x80, 0, 0, 0, 0], Reg::Byte(0)),
        ([0x78, 0xf9, 0x7a, 0x97, 255, 255, 255, 255], Reg::Word(15)),
    ] {
        let insn = decode_insn(&bytes, Target::H8SX, Mode::Maximum)
            .unwrap()
            .insn;
        assert!(
            matches!(insn.operands[0], Operand::Address(Ea::Indexed { index, .. }) if index == register)
        );
        assert_eq!(
            encode_insn(insn, Target::H8SX, Mode::Maximum).unwrap(),
            bytes
        );
    }
}

#[test]
fn longer_encodings_cover_every_register_and_extension_boundary() {
    let fields = include_bytes!("data/sx_fields.bin");
    assert_eq!(fields.len(), 163471 * 15);
    for record in fields.chunks_exact(15) {
        let bytes = &record[1..1 + usize::from(record[0])];
        let decoded = decode_insn(bytes, Target::H8SX, Mode::Maximum).unwrap();
        assert_eq!(decoded.len, bytes.len());
        assert_eq!(
            encode_insn(decoded.insn, Target::H8SX, Mode::Maximum).unwrap(),
            bytes
        );
    }
}
