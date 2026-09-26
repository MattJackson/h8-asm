//! Human-readable Renesas assembly for the typed H8/300 decoder subset.
//!
//! Register names, instruction suffixes, branch mnemonics, and the `$`
//! location-counter notation follow the Renesas assembler manual,
//! REJ10J2039 §11.1 and §11.2. No address context is needed: a branch
//! operand is rendered as an expression relative to its own instruction.

use super::decode::{ByteRegister, Instruction};

const CONDITIONS: [&str; 16] = [
    "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
    "BLT", "BGT", "BLE",
];

fn byte_register(register: ByteRegister) -> String {
    let code = register.code();
    if code < 8 {
        format!("R{code}H")
    } else {
        format!("R{}L", code - 8)
    }
}

// The H8/300 branch displacement is relative to the following two-byte
// instruction address. Renesas `$` denotes this instruction's start, so
// `$ + 2 + displacement` names its destination without an external PC.
fn branch_operand(displacement: i8) -> String {
    let offset = i16::from(displacement) + 2;
    if offset >= 0 {
        format!("$+{offset}:8")
    } else {
        format!("${offset}:8")
    }
}

/// Renders a decoded H8/300 instruction as one Renesas-style assembly line.
///
/// Relative branches use the Renesas `$` location counter. For example,
/// displacement `-2` renders as `$+0:8`, naming the branch instruction itself;
/// the suffix fixes the eight-bit branch format. The input is expected to
/// come from [`super::decode::decode`].
pub fn disassemble(instruction: Instruction) -> String {
    match instruction {
        Instruction::Nop => "NOP".into(),
        Instruction::Sleep => "SLEEP".into(),
        Instruction::Rts => "RTS".into(),
        Instruction::Rte => "RTE".into(),
        Instruction::AddByte {
            source,
            destination,
        } => {
            format!(
                "ADD.B {},{}",
                byte_register(source),
                byte_register(destination)
            )
        }
        Instruction::MovByte {
            source,
            destination,
        } => {
            format!(
                "MOV.B {},{}",
                byte_register(source),
                byte_register(destination)
            )
        }
        Instruction::Branch {
            condition,
            displacement,
        } => {
            let mnemonic = CONDITIONS
                .get(usize::from(condition))
                .copied()
                .unwrap_or("B??");
            format!("{mnemonic} {}", branch_operand(displacement))
        }
        Instruction::BranchSubroutine { displacement } => {
            format!("BSR {}", branch_operand(displacement))
        }
    }
}
