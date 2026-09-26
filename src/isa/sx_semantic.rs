//! Strict semantic decoding of H8SX control-flow and byte-immediate compares.
//!
//! These encodings follow REJ09B0102 §2.4 (CMP.B, Bcc, BSR, JMP, JSR).
//! This deliberately refuses all other instructions, so callers cannot
//! mistake an instruction-length guess for a full disassembly.

use crate::{Mode, Target};

/// The supported H8SX instruction meanings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SxInstruction {
    /// Compare an immediate byte with a byte register (A r imm).
    CompareByteImmediate {
        /// Destination byte-register code.
        register: u8,
        /// Compared literal.
        immediate: u8,
    },
    /// Eight-bit PC-relative conditional branch.
    Branch8 {
        /// Condition-code nibble.
        condition: u8,
        /// Offset from the following instruction.
        displacement: i8,
    },
    /// Sixteen-bit PC-relative conditional branch.
    Branch16 {
        /// Condition-code nibble.
        condition: u8,
        /// Offset from the following instruction.
        displacement: i16,
    },
    /// Eight-bit PC-relative subroutine call.
    BranchSubroutine8 {
        /// Offset from the following instruction.
        displacement: i8,
    },
    /// Sixteen-bit PC-relative subroutine call.
    BranchSubroutine16 {
        /// Offset from the following instruction.
        displacement: i16,
    },
    /// Direct 24-bit jump.
    Jump24 {
        /// Absolute 24-bit code address.
        address: u32,
    },
    /// Direct 24-bit subroutine call.
    Call24 {
        /// Absolute 24-bit code address.
        address: u32,
    },
    /// Return from subroutine.
    Return,
}

/// One decoded instruction with its exact byte length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SxDecoded {
    /// Instruction semantics.
    pub instruction: SxInstruction,
    /// Encoded byte length.
    pub len: usize,
}

/// Decode a narrow H8SX semantic subset. Unsupported/truncated encodings return `None`.
pub fn decode(bytes: &[u8], target: Target, _mode: Mode) -> Option<SxDecoded> {
    if target != Target::H8SX {
        return None;
    }
    let hi = *bytes.first()?;
    let lo = *bytes.get(1)?;
    let (instruction, len) = match (hi, lo) {
        (0xa0..=0xaf, imm) => (
            SxInstruction::CompareByteImmediate {
                register: hi & 15,
                immediate: imm,
            },
            2,
        ),
        (0x40..=0x4f, disp) if disp & 1 == 0 => (
            SxInstruction::Branch8 {
                condition: hi & 15,
                displacement: disp as i8,
            },
            2,
        ),
        (0x58, cond) if cond & 15 == 0 => {
            let disp = i16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]);
            if disp & 1 != 0 {
                return None;
            }
            (
                SxInstruction::Branch16 {
                    condition: cond >> 4,
                    displacement: disp,
                },
                4,
            )
        }
        (0x55, disp) if disp & 1 == 0 => (
            SxInstruction::BranchSubroutine8 {
                displacement: disp as i8,
            },
            2,
        ),
        (0x5c, 0x00) => {
            let disp = i16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]);
            if disp & 1 != 0 {
                return None;
            }
            (SxInstruction::BranchSubroutine16 { displacement: disp }, 4)
        }
        (0x5a, top) => (
            SxInstruction::Jump24 {
                address: u32::from_be_bytes([0, top, *bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x5e, top) => (
            SxInstruction::Call24 {
                address: u32::from_be_bytes([0, top, *bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x54, 0x70) => (SxInstruction::Return, 2),
        _ => return None,
    };
    Some(SxDecoded { instruction, len })
}
