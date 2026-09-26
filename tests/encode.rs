use h8_asm::{
    isa::{
        decode::{decode, ByteRegister, Instruction},
        encode::{encode, EncodeError},
    },
    Mode, Target,
};

#[test]
fn all_decoded_two_byte_forms_encode_identically() {
    let mut count = 0;
    for word in 0..=u16::MAX {
        let bytes = word.to_be_bytes();
        if let Ok(decoded) = decode(&bytes, Target::H8_300, Mode::Normal) {
            assert_eq!(decoded.len, 2);
            assert_eq!(
                encode(decoded.instruction, Target::H8_300, Mode::Normal),
                Ok(bytes)
            );
            count += 1;
        }
    }
    assert_eq!(count, 2692);
}

#[test]
fn invalid_operands_and_targets_are_refused() {
    let add = Instruction::AddByte {
        source: ByteRegister::from_code(8).unwrap(),
        destination: ByteRegister::from_code(15).unwrap(),
    };
    assert_eq!(encode(add, Target::H8_300, Mode::Normal), Ok([0x08, 0x8f]));
    assert_eq!(ByteRegister::from_code(16), None);
    assert_eq!(
        encode(
            Instruction::Branch {
                condition: 16,
                displacement: 0
            },
            Target::H8_300,
            Mode::Normal
        ),
        Err(EncodeError::OperandOutOfRange)
    );
    assert_eq!(
        encode(
            Instruction::Branch {
                condition: 0,
                displacement: 1
            },
            Target::H8_300,
            Mode::Normal
        ),
        Err(EncodeError::UnalignedBranch)
    );
    assert_eq!(
        encode(
            Instruction::BranchSubroutine { displacement: -1 },
            Target::H8_300,
            Mode::Normal
        ),
        Err(EncodeError::UnalignedBranch)
    );
    assert_eq!(
        encode(Instruction::Nop, Target::H8_300H, Mode::Normal),
        Err(EncodeError::UnsupportedTarget)
    );
    assert_eq!(
        encode(Instruction::Nop, Target::H8_300, Mode::Advanced),
        Err(EncodeError::UnsupportedTarget)
    );
}
