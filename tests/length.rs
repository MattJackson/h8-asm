use h8_asm::{isa::insn_len, Mode, Target};

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
        [11506, 53556, 474, 0, 0, 0],
        [7647, 56520, 1249, 120, 0, 0],
        [7295, 56872, 1249, 120, 0, 0],
        [7262, 56905, 1249, 120, 0, 0],
        [65532, 4, 0, 0, 0, 0],
    ]) {
        let mut counts = [0usize; 6];
        for first in 0..=u16::MAX {
            let mut bytes = [0; 10];
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
                if reg >= count { Some(4) } else { None }
            );
            assert_eq!(
                insn_len(&store, Target::H8S2000, Mode::Normal),
                if reg + count <= 7 { Some(4) } else { None }
            );
        }
    }
}
