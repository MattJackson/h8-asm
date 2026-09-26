use h8_asm::{
    isa::{
        sx_encode::{encode, SxEncodeError, SxEncoded},
        sx_semantic::{decode, SxInstruction},
    },
    Mode, Target,
};

fn roundtrip(instruction: SxInstruction, mode: Mode, expected: SxEncoded) {
    let encoded = encode(instruction, Target::H8SX, mode).unwrap();
    assert_eq!(encoded, expected);
    let decoded = decode(encoded.as_bytes(), Target::H8SX, mode).unwrap();
    assert_eq!(decoded.instruction, instruction);
    assert_eq!(decoded.len, encoded.as_bytes().len());
}

#[test]
fn two_byte_forms_roundtrip() {
    for mode in [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum] {
        for register in 0..=15 {
            for immediate in 0..=u8::MAX {
                roundtrip(
                    SxInstruction::MoveByteImmediate {
                        register,
                        immediate,
                    },
                    mode,
                    SxEncoded::Two([0xf0 | register, immediate]),
                );
                roundtrip(
                    SxInstruction::CompareByteImmediate {
                        register,
                        immediate,
                    },
                    mode,
                    SxEncoded::Two([0xa0 | register, immediate]),
                );
            }
        }
        for condition in 0..=15 {
            for displacement in (i8::MIN..=i8::MAX).step_by(2) {
                roundtrip(
                    SxInstruction::Branch8 {
                        condition,
                        displacement,
                    },
                    mode,
                    SxEncoded::Two([0x40 | condition, displacement as u8]),
                );
            }
        }
        for displacement in (i8::MIN..=i8::MAX).step_by(2) {
            roundtrip(
                SxInstruction::BranchSubroutine8 { displacement },
                mode,
                SxEncoded::Two([0x55, displacement as u8]),
            );
        }
        roundtrip(SxInstruction::Return, mode, SxEncoded::Two([0x54, 0x70]));
    }
}

#[test]
fn four_byte_forms_roundtrip() {
    for mode in [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum] {
        for register in 0..=15 {
            for address in [0u16, 0x2c24, 0xffff] {
                let [hi, lo] = address.to_be_bytes();
                roundtrip(
                    SxInstruction::LoadByteAbsolute16 { address, register },
                    mode,
                    SxEncoded::Four([0x6a, register, hi, lo]),
                );
                roundtrip(
                    SxInstruction::StoreByteAbsolute16 { address, register },
                    mode,
                    SxEncoded::Four([0x6a, 0x80 | register, hi, lo]),
                );
            }
        }
        for condition in 0..=15 {
            for displacement in [i16::MIN, -2, 0, 2, i16::MAX - 1] {
                let [hi, lo] = displacement.to_be_bytes();
                roundtrip(
                    SxInstruction::Branch16 {
                        condition,
                        displacement,
                    },
                    mode,
                    SxEncoded::Four([0x58, condition << 4, hi, lo]),
                );
            }
        }
        for displacement in [i16::MIN, -2, 0, 2, i16::MAX - 1] {
            let [hi, lo] = displacement.to_be_bytes();
            roundtrip(
                SxInstruction::BranchSubroutine16 { displacement },
                mode,
                SxEncoded::Four([0x5c, 0x00, hi, lo]),
            );
        }
        for address in [0u32, 0x1234, 0xffff, 0x12_3456, 0xff_ffff] {
            if mode == Mode::Normal && address > 0xffff {
                continue;
            }
            let [_, high, middle, low] = address.to_be_bytes();
            roundtrip(
                SxInstruction::Jump24 { address },
                mode,
                SxEncoded::Four([0x5a, high, middle, low]),
            );
            roundtrip(
                SxInstruction::Call24 { address },
                mode,
                SxEncoded::Four([0x5e, high, middle, low]),
            );
        }
    }
}

#[test]
fn rejects_invalid_operands() {
    let invalid = [
        (
            SxInstruction::MoveByteImmediate {
                register: 16,
                immediate: 0,
            },
            SxEncodeError::OperandOutOfRange,
        ),
        (
            SxInstruction::LoadByteAbsolute16 {
                address: 0,
                register: 16,
            },
            SxEncodeError::OperandOutOfRange,
        ),
        (
            SxInstruction::StoreByteAbsolute16 {
                address: 0,
                register: 16,
            },
            SxEncodeError::OperandOutOfRange,
        ),
        (
            SxInstruction::CompareByteImmediate {
                register: 16,
                immediate: 0,
            },
            SxEncodeError::OperandOutOfRange,
        ),
        (
            SxInstruction::Branch8 {
                condition: 16,
                displacement: 0,
            },
            SxEncodeError::OperandOutOfRange,
        ),
        (
            SxInstruction::Branch16 {
                condition: 16,
                displacement: 0,
            },
            SxEncodeError::OperandOutOfRange,
        ),
        (
            SxInstruction::Branch8 {
                condition: 0,
                displacement: 1,
            },
            SxEncodeError::UnalignedBranch,
        ),
        (
            SxInstruction::Branch16 {
                condition: 0,
                displacement: -1,
            },
            SxEncodeError::UnalignedBranch,
        ),
        (
            SxInstruction::BranchSubroutine8 { displacement: -1 },
            SxEncodeError::UnalignedBranch,
        ),
        (
            SxInstruction::BranchSubroutine16 { displacement: 1 },
            SxEncodeError::UnalignedBranch,
        ),
        (
            SxInstruction::Jump24 {
                address: 0x100_0000,
            },
            SxEncodeError::AddressOutOfRange,
        ),
        (
            SxInstruction::Call24 {
                address: 0x100_0000,
            },
            SxEncodeError::AddressOutOfRange,
        ),
    ];
    for (instruction, error) in invalid {
        assert_eq!(encode(instruction, Target::H8SX, Mode::Maximum), Err(error));
    }
    assert_eq!(
        encode(
            SxInstruction::Jump24 { address: 0x1_0000 },
            Target::H8SX,
            Mode::Normal
        ),
        Err(SxEncodeError::AddressOutOfRange)
    );
    assert_eq!(
        encode(SxInstruction::Return, Target::H8_300, Mode::Normal),
        Err(SxEncodeError::UnsupportedTarget)
    );
}
