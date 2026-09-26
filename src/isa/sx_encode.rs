//! Encoding for the strict H8SX subset in [`super::sx_semantic`].
//!
//! Forms follow REJ09B0102 §2.4: supported arithmetic/logic and MOV register
//! and immediate rows, absolute byte moves, Bcc, BSR, JMP, JSR, RTS.

use super::sx_semantic::SxInstruction;
use crate::{Mode, Target};

/// A complete two-, four-, or six-byte H8SX instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SxEncoded {
    /// One opcode word.
    Two([u8; 2]),
    /// Two opcode words.
    Four([u8; 4]),
    /// Three opcode words.
    Six([u8; 6]),
}

impl SxEncoded {
    /// Returns the exact instruction bytes without padding.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Two(bytes) => bytes,
            Self::Four(bytes) => bytes,
            Self::Six(bytes) => bytes,
        }
    }
}

/// Why a requested H8SX instruction cannot be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SxEncodeError {
    /// The requested target or operating mode is unsupported.
    UnsupportedTarget,
    /// An operand does not fit its field or its operation/size is unsupported.
    OperandOutOfRange,
    /// A branch displacement must be even.
    UnalignedBranch,
    /// The jump or call address does not fit the 24-bit operand field.
    AddressOutOfRange,
}

/// Encodes one supported H8SX instruction for `target` and `mode`.
///
/// Branch displacements are signed offsets from the instruction after this
/// one. Direct jump/call operands have a 24-bit field; in normal mode the
/// program area is 16-bit, so addresses above `0xffff` are refused.
///
/// # Panics
/// Panics on an internal encoder/decoder disagreement, before returning bytes.
/// Invalid user operands return [`SxEncodeError`] instead.
pub fn encode(
    instruction: SxInstruction,
    target: Target,
    mode: Mode,
) -> Result<SxEncoded, SxEncodeError> {
    if target != Target::H8SX {
        return Err(SxEncodeError::UnsupportedTarget);
    }
    let encoded = match instruction {
        SxInstruction::RegisterBinary {
            operation,
            size,
            source,
            destination,
        } => SxEncoded::Two(
            super::sx_register::encode(operation, size, source, destination)
                .ok_or(SxEncodeError::OperandOutOfRange)?,
        ),
        SxInstruction::AddByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0x80 | register, immediate])
        }
        SxInstruction::AddCarryByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0x90 | register, immediate])
        }
        SxInstruction::SubtractCarryByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0xb0 | register, immediate])
        }
        SxInstruction::OrByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0xc0 | register, immediate])
        }
        SxInstruction::XorByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0xd0 | register, immediate])
        }
        SxInstruction::AndByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0xe0 | register, immediate])
        }
        SxInstruction::MoveWordImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Four([
                0x79,
                register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
            ])
        }
        SxInstruction::MoveLongImmediate {
            register,
            immediate,
        } => {
            if register > 7 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Six([
                0x7a,
                register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
                immediate.to_be_bytes()[2],
                immediate.to_be_bytes()[3],
            ])
        }
        SxInstruction::AddWordImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Four([
                0x79,
                0x10 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
            ])
        }
        SxInstruction::AddLongImmediate {
            register,
            immediate,
        } => {
            if register > 7 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Six([
                0x7a,
                0x10 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
                immediate.to_be_bytes()[2],
                immediate.to_be_bytes()[3],
            ])
        }
        SxInstruction::CompareWordImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Four([
                0x79,
                0x20 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
            ])
        }
        SxInstruction::SubtractWordImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Four([
                0x79,
                0x30 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
            ])
        }
        SxInstruction::SubtractLongImmediate {
            register,
            immediate,
        } => {
            if register > 7 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Six([
                0x7a,
                0x30 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
                immediate.to_be_bytes()[2],
                immediate.to_be_bytes()[3],
            ])
        }
        SxInstruction::OrWordImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Four([
                0x79,
                0x40 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
            ])
        }
        SxInstruction::OrLongImmediate {
            register,
            immediate,
        } => {
            if register > 7 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Six([
                0x7a,
                0x40 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
                immediate.to_be_bytes()[2],
                immediate.to_be_bytes()[3],
            ])
        }
        SxInstruction::XorWordImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Four([
                0x79,
                0x50 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
            ])
        }
        SxInstruction::XorLongImmediate {
            register,
            immediate,
        } => {
            if register > 7 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Six([
                0x7a,
                0x50 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
                immediate.to_be_bytes()[2],
                immediate.to_be_bytes()[3],
            ])
        }
        SxInstruction::AndWordImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Four([
                0x79,
                0x60 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
            ])
        }
        SxInstruction::AndLongImmediate {
            register,
            immediate,
        } => {
            if register > 7 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Six([
                0x7a,
                0x60 | register,
                immediate.to_be_bytes()[0],
                immediate.to_be_bytes()[1],
                immediate.to_be_bytes()[2],
                immediate.to_be_bytes()[3],
            ])
        }

        SxInstruction::CompareLongImmediate {
            register,
            immediate,
        } => {
            if register > 7 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            let [a, b, c, d] = immediate.to_be_bytes();
            SxEncoded::Six([0x7a, 0x20 | register, a, b, c, d])
        }
        SxInstruction::MoveByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0xf0 | register, immediate])
        }
        SxInstruction::LoadByteAbsolute16 { address, register }
        | SxInstruction::StoreByteAbsolute16 { address, register } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            let [hi, lo] = address.to_be_bytes();
            let store = matches!(instruction, SxInstruction::StoreByteAbsolute16 { .. });
            SxEncoded::Four([0x6a, (if store { 0x80 } else { 0 }) | register, hi, lo])
        }
        SxInstruction::CompareByteImmediate {
            register,
            immediate,
        } => {
            if register > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            SxEncoded::Two([0xa0 | register, immediate])
        }
        SxInstruction::Branch8 {
            condition,
            displacement,
        } => {
            if condition > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            if displacement & 1 != 0 {
                return Err(SxEncodeError::UnalignedBranch);
            }
            SxEncoded::Two([0x40 | condition, displacement as u8])
        }
        SxInstruction::Branch16 {
            condition,
            displacement,
        } => {
            if condition > 15 {
                return Err(SxEncodeError::OperandOutOfRange);
            }
            if displacement & 1 != 0 {
                return Err(SxEncodeError::UnalignedBranch);
            }
            let [hi, lo] = displacement.to_be_bytes();
            SxEncoded::Four([0x58, condition << 4, hi, lo])
        }
        SxInstruction::BranchSubroutine8 { displacement } => {
            if displacement & 1 != 0 {
                return Err(SxEncodeError::UnalignedBranch);
            }
            SxEncoded::Two([0x55, displacement as u8])
        }
        SxInstruction::BranchSubroutine16 { displacement } => {
            if displacement & 1 != 0 {
                return Err(SxEncodeError::UnalignedBranch);
            }
            let [hi, lo] = displacement.to_be_bytes();
            SxEncoded::Four([0x5c, 0x00, hi, lo])
        }
        SxInstruction::Jump24 { address } | SxInstruction::Call24 { address } => {
            if address > 0x00ff_ffff || (mode == Mode::Normal && address > 0xffff) {
                return Err(SxEncodeError::AddressOutOfRange);
            }
            let [_, high, middle, low] = address.to_be_bytes();
            SxEncoded::Four([
                if matches!(instruction, SxInstruction::Jump24 { .. }) {
                    0x5a
                } else {
                    0x5e
                },
                high,
                middle,
                low,
            ])
        }
        SxInstruction::Return => SxEncoded::Two([0x54, 0x70]),
    };
    // A disagreement is an internal implementation defect. Never return bytes
    // that our independently dispatched decoder gives a different meaning.
    assert_eq!(
        super::sx_semantic::decode(encoded.as_bytes(), target, mode),
        Some(super::sx_semantic::SxDecoded {
            instruction,
            len: encoded.as_bytes().len()
        }),
        "encoder/decoder disagreement"
    );
    Ok(encoded)
}
