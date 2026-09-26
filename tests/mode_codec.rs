//! Context checks separate from the GNU mode-selector limitations.
use h8_asm::{
    isa::{decode_insn, encode_insn},
    Mode, Target,
};
use std::collections::BTreeSet;

#[test]
fn typed_two_byte_semantics_and_encodings_are_stable_in_every_supported_mode() {
    for target in [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
        Target::H8SX,
    ] {
        for word in 0..=u16::MAX {
            let bytes = word.to_be_bytes();
            let normal = decode_insn(&bytes, target, Mode::Normal);
            for mode in [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum] {
                let actual = decode_insn(&bytes, target, mode);
                if target.supports(mode) {
                    assert_eq!(actual, normal);
                    if let Ok(decoded) = actual {
                        assert_eq!(encode_insn(decoded.insn, target, mode).unwrap(), bytes);
                    }
                } else {
                    assert!(actual.is_err());
                }
            }
        }
    }
}

#[test]
fn inherited_extended_fields_round_trip_in_normal_and_advanced_modes() {
    for (target, expected) in [
        (Target::H8_300H, 280),
        (Target::H8S2000, 1274),
        (Target::H8S2600, 1274),
    ] {
        let mut cases = BTreeSet::new();
        for record in include_bytes!("data/sx_fields.bin").chunks_exact(15) {
            let bytes = &record[1..1 + usize::from(record[0])];
            if let Ok(decoded) = decode_insn(bytes, target, Mode::Advanced) {
                if decoded.len > 4 {
                    cases.insert(bytes[..decoded.len].to_vec());
                }
            }
        }
        assert_eq!(cases.len(), expected);
        for bytes in cases {
            for mode in [Mode::Normal, Mode::Advanced] {
                let decoded = decode_insn(&bytes, target, mode).unwrap();
                assert_eq!(encode_insn(decoded.insn, target, mode).unwrap(), bytes);
            }
        }
    }
}
