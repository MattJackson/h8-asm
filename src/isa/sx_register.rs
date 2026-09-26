//! Register arithmetic vocabulary and opcode map (REJ09B0102 §2.4).

/// Width of a register operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterSize {
    /// RnH/RnL, encoded in four bits.
    Byte,
    /// Rn/En, encoded in four bits.
    Word,
    /// ERn, encoded in three bits.
    Long,
}

/// A binary arithmetic, logical, or move operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperation {
    /// Add.
    Add,
    /// Add with carry.
    AddCarry,
    /// Subtract.
    Subtract,
    /// Subtract with carry.
    SubtractCarry,
    /// Compare.
    Compare,
    /// Move.
    Move,
    /// Bitwise inclusive OR.
    Or,
    /// Bitwise exclusive OR.
    Xor,
    /// Bitwise AND.
    And,
}

use BinaryOperation::*;
use RegisterSize::*;

// Only the single-word register rows. Prefixed long logical and word/long
// carry operations are not represented by this map and must be refused.
const ROWS: [(u8, RegisterSize, BinaryOperation); 20] = [
    (0x08, Byte, Add),
    (0x0c, Byte, Move),
    (0x0e, Byte, AddCarry),
    (0x14, Byte, Or),
    (0x15, Byte, Xor),
    (0x16, Byte, And),
    (0x18, Byte, Subtract),
    (0x1c, Byte, Compare),
    (0x1e, Byte, SubtractCarry),
    (0x09, Word, Add),
    (0x0d, Word, Move),
    (0x19, Word, Subtract),
    (0x1d, Word, Compare),
    (0x64, Word, Or),
    (0x65, Word, Xor),
    (0x66, Word, And),
    (0x0a, Long, Add),
    (0x0f, Long, Move),
    (0x1a, Long, Subtract),
    (0x1f, Long, Compare),
];

pub(super) fn decode(high: u8, low: u8) -> Option<(BinaryOperation, RegisterSize, u8, u8)> {
    let &(_, size, operation) = ROWS.iter().find(|row| row.0 == high)?;
    if size == Long {
        if low & 0x88 != 0x80 {
            return None;
        }
        Some((operation, size, (low >> 4) & 7, low & 7))
    } else {
        Some((operation, size, low >> 4, low & 15))
    }
}

pub(super) fn encode(
    operation: BinaryOperation,
    size: RegisterSize,
    source: u8,
    destination: u8,
) -> Option<[u8; 2]> {
    let &(opcode, _, _) = ROWS
        .iter()
        .find(|row| row.1 == size && row.2 == operation)?;
    let max = if size == Long { 7 } else { 15 };
    if source > max || destination > max {
        return None;
    }
    Some([
        opcode,
        source << 4 | destination | if size == Long { 0x80 } else { 0 },
    ])
}

pub(super) fn disassemble(
    operation: BinaryOperation,
    size: RegisterSize,
    source: u8,
    destination: u8,
) -> String {
    let mnemonic = match operation {
        Add => "ADD",
        AddCarry => "ADDX",
        Subtract => "SUB",
        SubtractCarry => "SUBX",
        Compare => "CMP",
        Move => "MOV",
        Or => "OR",
        Xor => "XOR",
        And => "AND",
    };
    let suffix = match size {
        Byte => "B",
        Word => "W",
        Long => "L",
    };
    format!(
        "{mnemonic}.{suffix} {},{}",
        register(size, source),
        register(size, destination)
    )
}

fn register(size: RegisterSize, code: u8) -> String {
    match size {
        Byte if code < 8 => format!("R{code}H"),
        Byte => format!("R{}L", code - 8),
        Word if code < 8 => format!("R{code}"),
        Word => format!("E{}", code - 8),
        Long => format!("ER{code}"),
    }
}
