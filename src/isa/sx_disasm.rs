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

fn word_register(register: u8) -> String {
    if register < 8 {
        format!("R{register}")
    } else {
        format!("E{}", register - 8)
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
        SxInstruction::RegisterBinary {
            operation,
            size,
            source,
            destination,
        } => super::sx_register::disassemble(operation, size, source, destination),
        SxInstruction::AddByteImmediate {
            register,
            immediate,
        } => format!("ADD.B #H'{immediate:02X}:8,{}", byte_register(register)),
        SxInstruction::AddCarryByteImmediate {
            register,
            immediate,
        } => format!("ADDX.B #H'{immediate:02X}:8,{}", byte_register(register)),
        SxInstruction::SubtractCarryByteImmediate {
            register,
            immediate,
        } => format!("SUBX.B #H'{immediate:02X}:8,{}", byte_register(register)),
        SxInstruction::OrByteImmediate {
            register,
            immediate,
        } => format!("OR.B #H'{immediate:02X}:8,{}", byte_register(register)),
        SxInstruction::XorByteImmediate {
            register,
            immediate,
        } => format!("XOR.B #H'{immediate:02X}:8,{}", byte_register(register)),
        SxInstruction::AndByteImmediate {
            register,
            immediate,
        } => format!("AND.B #H'{immediate:02X}:8,{}", byte_register(register)),
        SxInstruction::MoveWordImmediate {
            register,
            immediate,
        } => format!("MOV.W #H'{immediate:04X}:16,{}", word_register(register)),
        SxInstruction::MoveLongImmediate {
            register,
            immediate,
        } => format!("MOV.L #H'{immediate:08X}:32,ER{register}"),
        SxInstruction::AddWordImmediate {
            register,
            immediate,
        } => format!("ADD.W #H'{immediate:04X}:16,{}", word_register(register)),
        SxInstruction::AddLongImmediate {
            register,
            immediate,
        } => format!("ADD.L #H'{immediate:08X}:32,ER{register}"),
        SxInstruction::CompareWordImmediate {
            register,
            immediate,
        } => format!("CMP.W #H'{immediate:04X}:16,{}", word_register(register)),
        SxInstruction::SubtractWordImmediate {
            register,
            immediate,
        } => format!("SUB.W #H'{immediate:04X}:16,{}", word_register(register)),
        SxInstruction::SubtractLongImmediate {
            register,
            immediate,
        } => format!("SUB.L #H'{immediate:08X}:32,ER{register}"),
        SxInstruction::OrWordImmediate {
            register,
            immediate,
        } => format!("OR.W #H'{immediate:04X}:16,{}", word_register(register)),
        SxInstruction::OrLongImmediate {
            register,
            immediate,
        } => format!("OR.L #H'{immediate:08X}:32,ER{register}"),
        SxInstruction::XorWordImmediate {
            register,
            immediate,
        } => format!("XOR.W #H'{immediate:04X}:16,{}", word_register(register)),
        SxInstruction::XorLongImmediate {
            register,
            immediate,
        } => format!("XOR.L #H'{immediate:08X}:32,ER{register}"),
        SxInstruction::AndWordImmediate {
            register,
            immediate,
        } => format!("AND.W #H'{immediate:04X}:16,{}", word_register(register)),
        SxInstruction::AndLongImmediate {
            register,
            immediate,
        } => format!("AND.L #H'{immediate:08X}:32,ER{register}"),

        SxInstruction::CompareLongImmediate {
            register,
            immediate,
        } => format!("CMP.L #H'{immediate:08X}:32,ER{register}"),
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
