//! Encoding for the verified H8/300 semantic subset in [`super::decode`].
//!
//! Opcode forms follow ADE-602-025 §2: ADD.B, MOV.B, Bcc, BSR, NOP,
//! SLEEP, RTS, and RTE.

use super::decode::Instruction;
use crate::{Mode, Target};

/// Why an instruction cannot be encoded for the requested CPU context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncodeError {
    /// Only H8/300 normal mode is currently supported.
    UnsupportedTarget,
    /// A branch condition code is outside its four-bit field.
    OperandOutOfRange,
    /// Branch displacements must be even on H8/300.
    UnalignedBranch,
}

/// Encodes one verified H8/300 instruction into its two-byte big-endian form.
///
/// Branch displacements are signed offsets from the next instruction; callers
/// converting from an absolute target address must subtract `PC + 2` first.
/// An internal encoder/decoder disagreement panics before returning bytes;
/// invalid user operands return [`EncodeError`].
///
/// ```
/// use h8_asm::{isa::{decode::Instruction, encode::encode}, Mode, Target};
/// assert_eq!(encode(Instruction::BranchSubroutine { displacement: -2 },
///                   Target::H8_300, Mode::Normal), Ok([0x55, 0xfe]));
/// ```
pub fn encode(
    instruction: Instruction,
    target: Target,
    mode: Mode,
) -> Result<[u8; 2], EncodeError> {
    if target != Target::H8_300 || mode != Mode::Normal {
        return Err(EncodeError::UnsupportedTarget);
    }
    let bytes = match instruction {
        Instruction::Nop => [0x00, 0x00],
        Instruction::Sleep => [0x01, 0x80],
        Instruction::Rts => [0x54, 0x70],
        Instruction::Rte => [0x56, 0x70],
        Instruction::AddByte {
            source,
            destination,
        }
        | Instruction::MovByte {
            source,
            destination,
        } => {
            let (source, destination) = (source.code(), destination.code());
            [
                if matches!(instruction, Instruction::AddByte { .. }) {
                    0x08
                } else {
                    0x0c
                },
                source << 4 | destination,
            ]
        }
        Instruction::Branch {
            condition,
            displacement,
        } => {
            if condition > 15 {
                return Err(EncodeError::OperandOutOfRange);
            }
            if displacement & 1 != 0 {
                return Err(EncodeError::UnalignedBranch);
            }
            [0x40 | condition, displacement as u8]
        }
        Instruction::BranchSubroutine { displacement } => {
            if displacement & 1 != 0 {
                return Err(EncodeError::UnalignedBranch);
            }
            [0x55, displacement as u8]
        }
    };
    assert_eq!(
        super::decode::decode(&bytes, target, mode),
        Ok(super::decode::DecodedInstruction {
            instruction,
            len: bytes.len()
        }),
        "encoder/decoder disagreement"
    );
    Ok(bytes)
}
