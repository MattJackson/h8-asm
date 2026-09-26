//! Renesas-style assembly for the current typed H8SX semantic subset.
//!
//! Syntax follows REJ10J2039 §11: `$` is the current location counter,
//! `:8`/`:16` fix PC-relative branch width, and `:24` fixes the width of
//! direct jump and call operands.

use super::sx_semantic::SxInstruction;

const CONDITIONS: [&str; 16] = [
    "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
    "BLT", "BGT", "BLE",
];

fn branch_operand(displacement: i16, instruction_len: i32, width: u8) -> String {
    let offset = i32::from(displacement) + instruction_len;
    if offset >= 0 {
        format!("$+{offset}:{width}")
    } else {
        format!("${offset}:{width}")
    }
}

fn byte_register(register: u8) -> String {
    if register < 8 {
        format!("R{register}H")
    } else {
        format!("R{}L", register - 8)
    }
}

fn branch_mnemonic(condition: u8) -> &'static str {
    CONDITIONS
        .get(usize::from(condition))
        .copied()
        .unwrap_or("B??")
}

/// Renders a typed H8SX instruction as a single assembly line.
///
/// The branch operand is relative to the current instruction's `$` location
/// counter. The instruction length is added to the decoded displacement,
/// which is measured from the following instruction. This function does not
/// need a load address, and callers can render decoded instructions in
/// isolation.
pub fn disassemble(instruction: SxInstruction) -> String {
    match instruction {
        SxInstruction::MoveByteImmediate {
            register,
            immediate,
        } => format!("MOV.B #H'{immediate:02X},{}", byte_register(register)),
        SxInstruction::LoadByteAbsolute16 { address, register } => {
            format!("MOV.B @H'{address:04X}:16,{}", byte_register(register))
        }
        SxInstruction::StoreByteAbsolute16 { address, register } => {
            format!("MOV.B {},@H'{address:04X}:16", byte_register(register))
        }
        SxInstruction::CompareByteImmediate {
            register,
            immediate,
        } => {
            format!("CMP.B #H'{immediate:02X},{}", byte_register(register))
        }
        SxInstruction::Branch8 {
            condition,
            displacement,
        } => format!(
            "{} {}",
            branch_mnemonic(condition),
            branch_operand(i16::from(displacement), 2, 8)
        ),
        SxInstruction::Branch16 {
            condition,
            displacement,
        } => format!(
            "{} {}",
            branch_mnemonic(condition),
            branch_operand(displacement, 4, 16)
        ),
        SxInstruction::BranchSubroutine8 { displacement } => {
            format!("BSR {}", branch_operand(i16::from(displacement), 2, 8))
        }
        SxInstruction::BranchSubroutine16 { displacement } => {
            format!("BSR {}", branch_operand(displacement, 4, 16))
        }
        SxInstruction::Jump24 { address } => format!("JMP @H'{address:06X}:24"),
        SxInstruction::Call24 { address } => format!("JSR @H'{address:06X}:24"),
        SxInstruction::Return => "RTS".into(),
    }
}
