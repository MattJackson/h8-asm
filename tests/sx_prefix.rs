use h8_asm::{isa::insn_len, Mode, Target};

fn check(bytes: &[u8], expected: Option<usize>) {
    assert_eq!(
        insn_len(bytes, Target::H8SX, Mode::Normal),
        expected,
        "{bytes:02x?}"
    );
    if let Some(length) = expected {
        for end in 0..length {
            assert_eq!(insn_len(&bytes[..end], Target::H8SX, Mode::Normal), None);
        }
    }
}

#[test]
fn control_register_memory_prefixes() {
    for prefix in [0x40, 0x41] {
        for register in 0..8 {
            for second_high in [0x69, 0x6d] {
                for direction in [0, 8] {
                    check(
                        &[0x01, prefix, second_high, direction << 4 | register << 4],
                        Some(4),
                    );
                }
            }
            for direction in [0, 8] {
                check(
                    &[
                        0x01,
                        prefix,
                        0x6f,
                        direction << 4 | register << 4,
                        0x12,
                        0x34,
                    ],
                    Some(6),
                );
            }
        }
        for second_low in [0x00, 0x20, 0x80, 0xa0] {
            let length = if second_low & 0x20 == 0 { 6 } else { 8 };
            let mut bytes = vec![0x01, prefix, 0x6b, second_low, 0x12, 0x34];
            if length == 8 {
                bytes.extend([0x56, 0x78]);
            }
            check(&bytes, Some(length));
        }
        check(&[0x01, prefix, 0x69, 0x11], None);
        check(&[0x01, prefix, 0x6d, 0x11], None);
        check(&[0x01, prefix, 0x6f, 0x11, 0, 0], None);
        check(&[0x01, prefix, 0x6b, 0x01, 0, 0], None);
    }
    check(&[0x01, 0x41, 0x07, 0x12], Some(4));
    check(&[0x01, 0x40, 0x07, 0x12], None);
}

#[test]
fn multiple_register_and_mac_prefixes() {
    for count_minus_one in 1..=3u8 {
        for register in 0..8u8 {
            let prefix = count_minus_one << 4;
            let load = [0x01, prefix, 0x6d, 0x70 | register];
            let store = [0x01, prefix, 0x6d, 0xf0 | register];
            check(
                &load,
                if register >= count_minus_one {
                    Some(4)
                } else {
                    None
                },
            );
            check(
                &store,
                if register + count_minus_one < 8 {
                    Some(4)
                } else {
                    None
                },
            );
        }
    }
    for source in 0..8u8 {
        for destination in 0..8u8 {
            check(&[0x01, 0x60, 0x6d, source << 4 | destination], Some(4));
        }
    }
    check(&[0x01, 0x60, 0x6d, 0x80], None);
    check(&[0x01, 0x60, 0x6d, 0x08], None);
}
