use h8_asm::{
    isa::{
        decode::{decode, DecodeError, Instruction},
        insn_len,
    },
    Mode, Target,
};

#[test]
fn fixed_and_register_encodings() {
    for code in 0..=u8::MAX {
        assert_eq!(
            h8_asm::isa::decode::ByteRegister::from_code(code).map(|register| register.code()),
            if code < 16 { Some(code) } else { None }
        );
    }
    for (bytes, expected) in [
        ([0x00, 0x00], Instruction::Nop),
        ([0x01, 0x80], Instruction::Sleep),
        ([0x54, 0x70], Instruction::Rts),
        ([0x56, 0x70], Instruction::Rte),
    ] {
        let decoded = decode(&bytes, Target::H8_300, Mode::Normal).unwrap();
        assert_eq!(decoded.instruction, expected);
        assert_eq!(decoded.len, 2);
        assert_eq!(
            insn_len(&bytes, Target::H8_300, Mode::Normal),
            Some(decoded.len)
        );
    }

    for high in [0x08, 0x0c] {
        for low in 0..=u8::MAX {
            let bytes = [high, low, 0xff];
            let decoded = decode(&bytes, Target::H8_300, Mode::Normal).unwrap();
            let (source, destination) = match decoded.instruction {
                Instruction::AddByte {
                    source,
                    destination,
                }
                | Instruction::MovByte {
                    source,
                    destination,
                } => (source, destination),
                _ => panic!("unexpected instruction"),
            };
            assert_eq!(source.code(), low >> 4);
            assert_eq!(destination.code(), low & 15);
            assert_eq!(decoded.len, 2);
        }
    }
}

#[test]
fn signed_relative_branches() {
    for condition in 0..=15 {
        for (encoded, signed) in [(0x80, -128), (0xfe, -2), (0, 0), (0x7e, 126)] {
            let decoded =
                decode(&[0x40 | condition, encoded], Target::H8_300, Mode::Normal).unwrap();
            assert_eq!(
                decoded.instruction,
                Instruction::Branch {
                    condition,
                    displacement: signed
                }
            );
        }
    }
    assert_eq!(
        decode(&[0x55, 0xfe], Target::H8_300, Mode::Normal)
            .unwrap()
            .instruction,
        Instruction::BranchSubroutine { displacement: -2 }
    );
}

#[test]
fn refuses_truncated_unsupported_and_odd_branches() {
    assert_eq!(
        decode(&[], Target::H8_300, Mode::Normal),
        Err(DecodeError::Truncated)
    );
    assert_eq!(
        decode(&[0x08], Target::H8_300, Mode::Normal),
        Err(DecodeError::Truncated)
    );
    assert_eq!(
        decode(&[0x09, 0x00], Target::H8_300, Mode::Normal),
        Err(DecodeError::UnsupportedEncoding)
    );
    assert_eq!(
        decode(&[0x40, 0x01], Target::H8_300, Mode::Normal),
        Err(DecodeError::UnsupportedEncoding)
    );
    assert_eq!(
        decode(&[0x55, 0xff], Target::H8_300, Mode::Normal),
        Err(DecodeError::UnsupportedEncoding)
    );
    assert_eq!(
        decode(&[0x00, 0x00], Target::H8_300H, Mode::Normal),
        Err(DecodeError::UnsupportedTarget)
    );
    assert_eq!(
        decode(&[0x00, 0x00], Target::H8_300, Mode::Advanced),
        Err(DecodeError::UnsupportedTarget)
    );
}
