//! Strict semantic decoding of selected H8SX control-flow and register forms.
//!
//! These encodings follow REJ09B0102 §2.4: arithmetic/logic and MOV register
//! rows, immediate-register rows, absolute byte moves, Bcc, BSR, JMP, JSR, RTS.
//! This deliberately refuses all other instructions, so callers cannot
//! mistake an instruction-length guess for a full disassembly.

use crate::{Mode, Target};

/// The supported H8SX instruction meanings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SxInstruction {
    /// Single-word arithmetic, logic, or move between registers (§2.4).
    RegisterBinary {
        /// Operation to perform.
        operation: super::sx_register::BinaryOperation,
        /// Width of both register operands.
        size: super::sx_register::RegisterSize,
        /// Source register code.
        source: u8,
        /// Destination register code.
        destination: u8,
    },
    /// ADD.B with a 8-bit literal (REJ09B0102 §2.4).
    AddByteImmediate {
        /// Destination register code (R0H–R7H/R0L–R7L).
        register: u8,
        /// Literal bit pattern.
        immediate: u8,
    },
    /// ADDX.B with a 8-bit literal (REJ09B0102 §2.4).
    AddCarryByteImmediate {
        /// Destination register code (R0H–R7H/R0L–R7L).
        register: u8,
        /// Literal bit pattern.
        immediate: u8,
    },
    /// SUBX.B with a 8-bit literal (REJ09B0102 §2.4).
    SubtractCarryByteImmediate {
        /// Destination register code (R0H–R7H/R0L–R7L).
        register: u8,
        /// Literal bit pattern.
        immediate: u8,
    },
    /// OR.B with a 8-bit literal (REJ09B0102 §2.4).
    OrByteImmediate {
        /// Destination register code (R0H–R7H/R0L–R7L).
        register: u8,
        /// Literal bit pattern.
        immediate: u8,
    },
    /// XOR.B with a 8-bit literal (REJ09B0102 §2.4).
    XorByteImmediate {
        /// Destination register code (R0H–R7H/R0L–R7L).
        register: u8,
        /// Literal bit pattern.
        immediate: u8,
    },
    /// AND.B with a 8-bit literal (REJ09B0102 §2.4).
    AndByteImmediate {
        /// Destination register code (R0H–R7H/R0L–R7L).
        register: u8,
        /// Literal bit pattern.
        immediate: u8,
    },
    /// MOV.W with a 16-bit literal (REJ09B0102 §2.4).
    MoveWordImmediate {
        /// Destination register code (R0–R7/E0–E7).
        register: u8,
        /// Literal bit pattern.
        immediate: u16,
    },
    /// MOV.L with a 32-bit literal (REJ09B0102 §2.4).
    MoveLongImmediate {
        /// Destination register code (ER0–ER7).
        register: u8,
        /// Literal bit pattern.
        immediate: u32,
    },
    /// ADD.W with a 16-bit literal (REJ09B0102 §2.4).
    AddWordImmediate {
        /// Destination register code (R0–R7/E0–E7).
        register: u8,
        /// Literal bit pattern.
        immediate: u16,
    },
    /// ADD.L with a 32-bit literal (REJ09B0102 §2.4).
    AddLongImmediate {
        /// Destination register code (ER0–ER7).
        register: u8,
        /// Literal bit pattern.
        immediate: u32,
    },
    /// CMP.W with a 16-bit literal (REJ09B0102 §2.4).
    CompareWordImmediate {
        /// Destination register code (R0–R7/E0–E7).
        register: u8,
        /// Literal bit pattern.
        immediate: u16,
    },
    /// SUB.W with a 16-bit literal (REJ09B0102 §2.4).
    SubtractWordImmediate {
        /// Destination register code (R0–R7/E0–E7).
        register: u8,
        /// Literal bit pattern.
        immediate: u16,
    },
    /// SUB.L with a 32-bit literal (REJ09B0102 §2.4).
    SubtractLongImmediate {
        /// Destination register code (ER0–ER7).
        register: u8,
        /// Literal bit pattern.
        immediate: u32,
    },
    /// OR.W with a 16-bit literal (REJ09B0102 §2.4).
    OrWordImmediate {
        /// Destination register code (R0–R7/E0–E7).
        register: u8,
        /// Literal bit pattern.
        immediate: u16,
    },
    /// OR.L with a 32-bit literal (REJ09B0102 §2.4).
    OrLongImmediate {
        /// Destination register code (ER0–ER7).
        register: u8,
        /// Literal bit pattern.
        immediate: u32,
    },
    /// XOR.W with a 16-bit literal (REJ09B0102 §2.4).
    XorWordImmediate {
        /// Destination register code (R0–R7/E0–E7).
        register: u8,
        /// Literal bit pattern.
        immediate: u16,
    },
    /// XOR.L with a 32-bit literal (REJ09B0102 §2.4).
    XorLongImmediate {
        /// Destination register code (ER0–ER7).
        register: u8,
        /// Literal bit pattern.
        immediate: u32,
    },
    /// AND.W with a 16-bit literal (REJ09B0102 §2.4).
    AndWordImmediate {
        /// Destination register code (R0–R7/E0–E7).
        register: u8,
        /// Literal bit pattern.
        immediate: u16,
    },
    /// AND.L with a 32-bit literal (REJ09B0102 §2.4).
    AndLongImmediate {
        /// Destination register code (ER0–ER7).
        register: u8,
        /// Literal bit pattern.
        immediate: u32,
    },

    /// Compare a 32-bit immediate with an ER register.
    CompareLongImmediate {
        /// Destination ER register number.
        register: u8,
        /// Compared literal.
        immediate: u32,
    },
    /// Load an immediate byte into a byte register.
    MoveByteImmediate {
        /// Destination byte-register code.
        register: u8,
        /// Immediate byte.
        immediate: u8,
    },
    /// Load a byte from a 16-bit absolute address into a byte register.
    LoadByteAbsolute16 {
        /// The absolute 16-bit memory address.
        address: u16,
        /// Destination byte-register code.
        register: u8,
    },
    /// Store a byte register at a 16-bit absolute address.
    StoreByteAbsolute16 {
        /// The absolute 16-bit memory address.
        address: u16,
        /// Source byte-register code.
        register: u8,
    },
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
    if let Some((operation, size, source, destination)) = super::sx_register::decode(hi, lo) {
        return Some(SxDecoded {
            instruction: SxInstruction::RegisterBinary {
                operation,
                size,
                source,
                destination,
            },
            len: 2,
        });
    }
    let (instruction, len) = match (hi, lo) {
        (0x80..=0x8f, immediate) => (
            SxInstruction::AddByteImmediate {
                register: hi & 15,
                immediate,
            },
            2,
        ),
        (0x90..=0x9f, immediate) => (
            SxInstruction::AddCarryByteImmediate {
                register: hi & 15,
                immediate,
            },
            2,
        ),
        (0xb0..=0xbf, immediate) => (
            SxInstruction::SubtractCarryByteImmediate {
                register: hi & 15,
                immediate,
            },
            2,
        ),
        (0xc0..=0xcf, immediate) => (
            SxInstruction::OrByteImmediate {
                register: hi & 15,
                immediate,
            },
            2,
        ),
        (0xd0..=0xdf, immediate) => (
            SxInstruction::XorByteImmediate {
                register: hi & 15,
                immediate,
            },
            2,
        ),
        (0xe0..=0xef, immediate) => (
            SxInstruction::AndByteImmediate {
                register: hi & 15,
                immediate,
            },
            2,
        ),
        (0x79, 0x00..=0x0f) => (
            SxInstruction::MoveWordImmediate {
                register: lo & 15,
                immediate: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x7a, 0x00..=0x07) => (
            SxInstruction::MoveLongImmediate {
                register: lo & 7,
                immediate: u32::from_be_bytes([
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                    *bytes.get(4)?,
                    *bytes.get(5)?,
                ]),
            },
            6,
        ),
        (0x79, 0x10..=0x1f) => (
            SxInstruction::AddWordImmediate {
                register: lo & 15,
                immediate: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x7a, 0x10..=0x17) => (
            SxInstruction::AddLongImmediate {
                register: lo & 7,
                immediate: u32::from_be_bytes([
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                    *bytes.get(4)?,
                    *bytes.get(5)?,
                ]),
            },
            6,
        ),
        (0x79, 0x20..=0x2f) => (
            SxInstruction::CompareWordImmediate {
                register: lo & 15,
                immediate: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x79, 0x30..=0x3f) => (
            SxInstruction::SubtractWordImmediate {
                register: lo & 15,
                immediate: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x7a, 0x30..=0x37) => (
            SxInstruction::SubtractLongImmediate {
                register: lo & 7,
                immediate: u32::from_be_bytes([
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                    *bytes.get(4)?,
                    *bytes.get(5)?,
                ]),
            },
            6,
        ),
        (0x79, 0x40..=0x4f) => (
            SxInstruction::OrWordImmediate {
                register: lo & 15,
                immediate: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x7a, 0x40..=0x47) => (
            SxInstruction::OrLongImmediate {
                register: lo & 7,
                immediate: u32::from_be_bytes([
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                    *bytes.get(4)?,
                    *bytes.get(5)?,
                ]),
            },
            6,
        ),
        (0x79, 0x50..=0x5f) => (
            SxInstruction::XorWordImmediate {
                register: lo & 15,
                immediate: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x7a, 0x50..=0x57) => (
            SxInstruction::XorLongImmediate {
                register: lo & 7,
                immediate: u32::from_be_bytes([
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                    *bytes.get(4)?,
                    *bytes.get(5)?,
                ]),
            },
            6,
        ),
        (0x79, 0x60..=0x6f) => (
            SxInstruction::AndWordImmediate {
                register: lo & 15,
                immediate: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
            },
            4,
        ),
        (0x7a, 0x60..=0x67) => (
            SxInstruction::AndLongImmediate {
                register: lo & 7,
                immediate: u32::from_be_bytes([
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                    *bytes.get(4)?,
                    *bytes.get(5)?,
                ]),
            },
            6,
        ),

        (0x7a, 0x20..=0x27) => (
            SxInstruction::CompareLongImmediate {
                register: lo & 7,
                immediate: u32::from_be_bytes([
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                    *bytes.get(4)?,
                    *bytes.get(5)?,
                ]),
            },
            6,
        ),
        (0xf0..=0xff, imm) => (
            SxInstruction::MoveByteImmediate {
                register: hi & 15,
                immediate: imm,
            },
            2,
        ),
        (0x6a, 0x00..=0x0f) => (
            SxInstruction::LoadByteAbsolute16 {
                address: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
                register: lo & 15,
            },
            4,
        ),
        (0x6a, 0x80..=0x8f) => (
            SxInstruction::StoreByteAbsolute16 {
                address: u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]),
                register: lo & 15,
            },
            4,
        ),
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
