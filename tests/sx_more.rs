use h8_asm::{isa::insn_len, Mode, Target};

#[test]
fn mac_and_long_control_registers() {
    for high in [0x02, 0x03] {
        for group in [0x20, 0x30, 0x60, 0x70] {
            for register in 0..8 {
                let bytes = [high, group | register];
                assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(2));
            }
            assert_eq!(
                insn_len(&[high, group | 8], Target::H8SX, Mode::Normal),
                None
            );
        }
    }
}

#[test]
fn five_bit_shift_counts() {
    for count in 1..=31u8 {
        for opcode in [0x10, 0x11] {
            for register in 0..16u8 {
                for size in [0x00, 0x10] {
                    let bytes = [0x03, 0x80 | count, opcode, size | register];
                    assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
                    for end in 0..4 {
                        assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
                    }
                }
            }
            for register in 0..8u8 {
                let bytes = [0x03, 0x80 | count, opcode, 0x30 | register];
                assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
            }
        }
    }
    assert_eq!(
        insn_len(&[0x03, 0x80, 0x10, 0x00], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x03, 0x81, 0x10, 0x38], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x03, 0x81, 0x12, 0x00], Target::H8SX, Mode::Normal),
        None
    );
}

#[test]
fn byte_immediate_absolute_memory() {
    for (first, extension, length) in [
        (0x18u8, &[0x12, 0x34][..], 6),
        (0x38u8, &[0x12, 0x34, 0x56, 0x78][..], 8),
    ] {
        for opcode in [0x80, 0xa0, 0xa1, 0xc0, 0xd0, 0xe0] {
            let mut bytes = vec![0x6a, first];
            bytes.extend_from_slice(extension);
            bytes.extend([opcode, 0x12]);
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(length));
            for end in 0..length {
                assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
            }
        }
        let mut invalid = vec![0x6a, first];
        invalid.extend_from_slice(extension);
        invalid.extend([0x90, 0x12]);
        assert_eq!(insn_len(&invalid, Target::H8SX, Mode::Normal), None);
    }
}
