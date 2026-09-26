use h8_asm::{isa::insn_len, Mode, Target};

#[test]
fn eepmov_exact_words() {
    for first in [0x5c, 0xd4] {
        let bytes = [0x7b, first, 0x59, 0x8f];
        assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
        for end in 0..4 {
            assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
        }
        assert_eq!(
            insn_len(&[0x7b, first, 0x59, 0x00], Target::H8SX, Mode::Normal),
            None
        );
    }
}

#[test]
fn basic_immediate_memory_bits() {
    for bit in 0..8u8 {
        for register in 0..8u8 {
            for (first_high, opcodes) in [
                (0x7c, &[0x73, 0x74, 0x75, 0x76, 0x77][..]),
                (0x7d, &[0x70, 0x71, 0x72][..]),
            ] {
                for opcode in opcodes {
                    let bytes = [first_high, register << 4, *opcode, bit << 4];
                    assert_eq!(
                        insn_len(&bytes, Target::H8SX, Mode::Normal),
                        Some(4),
                        "{bytes:02x?}"
                    );
                    for end in 0..4 {
                        assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
                    }
                }
            }
        }
        for address in [0x00, 0x12, 0xff] {
            for (first_high, opcodes) in [
                (0x7e, &[0x73, 0x74, 0x75, 0x76, 0x77][..]),
                (0x7f, &[0x70, 0x71, 0x72][..]),
            ] {
                for opcode in opcodes {
                    let bytes = [first_high, address, *opcode, bit << 4];
                    assert_eq!(
                        insn_len(&bytes, Target::H8SX, Mode::Normal),
                        Some(4),
                        "{bytes:02x?}"
                    );
                }
            }
        }
    }
    assert_eq!(
        insn_len(&[0x7c, 0x08, 0x73, 0x10], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x7d, 0x20, 0x70, 0x11], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x7e, 0x12, 0x70, 0x10], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x7f, 0x12, 0x73, 0x10], Target::H8SX, Mode::Normal),
        None
    );
}
