use h8_asm::{isa::insn_len, Mode, Target};

#[test]
fn register_selected_memory_bit_operations() {
    for register in 0..16u8 {
        for memory_register in 0..8u8 {
            for (first, opcodes) in [(0x7c, &[0x63][..]), (0x7d, &[0x60, 0x61, 0x62][..])] {
                for opcode in opcodes {
                    let bytes = [first, memory_register << 4, *opcode, register << 4];
                    assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
                    for end in 0..4 {
                        assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
                    }
                }
            }
        }
        for address in [0x00, 0x12, 0xff] {
            for (first, opcodes) in [(0x7e, &[0x63][..]), (0x7f, &[0x60, 0x61, 0x62][..])] {
                for opcode in opcodes {
                    let bytes = [first, address, *opcode, register << 4];
                    assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
                }
            }
        }
    }
    for memory_register in 0..8u8 {
        for bit in 0..16u8 {
            let bytes = [0x7d, memory_register << 4, 0x67, bit << 4];
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Normal), Some(4));
        }
    }
    assert_eq!(
        insn_len(&[0x7d, 0x20, 0x63, 0x90], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x7c, 0x20, 0x60, 0x90], Target::H8SX, Mode::Normal),
        None
    );
    assert_eq!(
        insn_len(&[0x7c, 0x20, 0x63, 0x91], Target::H8SX, Mode::Normal),
        None
    );
}
