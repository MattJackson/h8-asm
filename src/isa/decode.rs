//! Semantic decoding for a verified subset of the H8/300 instruction set.
//!
//! Encodings come from ADE-602-025 §2 instruction-format tables: ADD.B,
//! MOV.B, Bcc, BSR, NOP, SLEEP, RTS, and RTE. Other encodings are refused.

use crate::{Mode, Target};

/// An H8/300 byte register: 0–7 denote R0H–R7H, 8–15 denote R0L–R7L.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRegister(u8);

impl ByteRegister {
    /// The four-bit register number used in the instruction code.
    pub fn code(self) -> u8 {
        self.0
    }
}

/// Semantics of a recognized instruction; relative displacements use the PC
/// after this two-byte instruction as their base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instruction {
    /// No operation (0000).
    Nop,
    /// Enter the sleep state (0180).
    Sleep,
    /// Return from a subroutine (5470).
    Rts,
    /// Return from an exception (5670).
    Rte,
    /// Add a byte register to a byte register (08 rs rd).
    AddByte {
        /// Source byte register.
        source: ByteRegister,
        /// Destination byte register, which receives the result.
        destination: ByteRegister,
    },
    /// Move a byte register to a byte register (0c rs rd).
    MovByte {
        /// Source byte register.
        source: ByteRegister,
        /// Destination byte register.
        destination: ByteRegister,
    },
    /// Conditional PC-relative branch (4c disp), where `condition` is the
    /// four-bit Bcc condition code from ADE-602-025 §2 Bcc.
    Branch {
        /// Four-bit Bcc condition code.
        condition: u8,
        /// Signed displacement from the following instruction.
        displacement: i8,
    },
    /// PC-relative branch to subroutine (55 disp).
    BranchSubroutine {
        /// Signed displacement from the following instruction.
        displacement: i8,
    },
}

/// A decoded H8/300 instruction and its encoded length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecodedInstruction {
    /// Instruction meaning.
    pub instruction: Instruction,
    /// Number of bytes consumed from the input.
    pub len: usize,
}

/// Why an instruction could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodeError {
    /// This decoder currently covers H8/300 normal mode only.
    UnsupportedTarget,
    /// Fewer than two instruction bytes were supplied.
    Truncated,
    /// The first word is outside the implemented semantic subset, or has an
    /// invalid branch displacement.
    UnsupportedEncoding,
}

/// Decodes one instruction from the start of `bytes`, ignoring trailing data.
///
/// The branch displacement is signed and relative to the address immediately
/// after the instruction. The H8/300 manual requires an even displacement;
/// odd branch displacements are refused rather than given misleading targets.
///
/// ```
/// use h8_asm::{isa::decode::{decode, Instruction}, Mode, Target};
/// let decoded = decode(&[0x08, 0x19], Target::H8_300, Mode::Normal).unwrap();
/// assert!(matches!(decoded.instruction, Instruction::AddByte { .. }));
/// assert_eq!(decoded.len, 2);
/// ```
pub fn decode(bytes: &[u8], target: Target, mode: Mode) -> Result<DecodedInstruction, DecodeError> {
    if target != Target::H8_300 || mode != Mode::Normal {
        return Err(DecodeError::UnsupportedTarget);
    }
    let (&high, rest) = bytes.split_first().ok_or(DecodeError::Truncated)?;
    let &low = rest.first().ok_or(DecodeError::Truncated)?;
    let registers = || (ByteRegister(low >> 4), ByteRegister(low & 0x0f));
    let instruction = match (high, low) {
        (0x00, 0x00) => Instruction::Nop,
        (0x01, 0x80) => Instruction::Sleep,
        (0x54, 0x70) => Instruction::Rts,
        (0x56, 0x70) => Instruction::Rte,
        (0x08, _) => {
            let (source, destination) = registers();
            Instruction::AddByte {
                source,
                destination,
            }
        }
        (0x0c, _) => {
            let (source, destination) = registers();
            Instruction::MovByte {
                source,
                destination,
            }
        }
        (0x40..=0x4f, displacement) if displacement & 1 == 0 => Instruction::Branch {
            condition: high & 0x0f,
            displacement: displacement as i8,
        },
        (0x55, displacement) if displacement & 1 == 0 => Instruction::BranchSubroutine {
            displacement: displacement as i8,
        },
        _ => return Err(DecodeError::UnsupportedEncoding),
    };
    Ok(DecodedInstruction {
        instruction,
        len: 2,
    })
}
