use h8_asm::{
    isa::{
        sx_disasm::disassemble,
        sx_encode::encode,
        sx_register::{BinaryOperation::*, RegisterSize::*},
        sx_semantic::{decode, SxInstruction},
    },
    Mode, Target,
};

#[test]
fn register_text_and_refusals() {
    let cases = [
        (Add, Byte, 0, 15, [0x08, 0x0f], "ADD.B R0H,R7L"),
        (AddCarry, Byte, 8, 7, [0x0e, 0x87], "ADDX.B R0L,R7H"),
        (Subtract, Word, 15, 0, [0x19, 0xf0], "SUB.W E7,R0"),
        (SubtractCarry, Byte, 15, 8, [0x1e, 0xf8], "SUBX.B R7L,R0L"),
        (Compare, Long, 7, 0, [0x1f, 0xf0], "CMP.L ER7,ER0"),
        (Move, Word, 0, 8, [0x0d, 0x08], "MOV.W R0,E0"),
        (Or, Byte, 0, 0, [0x14, 0x00], "OR.B R0H,R0H"),
        (Xor, Word, 8, 8, [0x65, 0x88], "XOR.W E0,E0"),
        (And, Byte, 7, 15, [0x16, 0x7f], "AND.B R7H,R7L"),
    ];
    for (operation, size, source, destination, bytes, text) in cases {
        let instruction = SxInstruction::RegisterBinary {
            operation,
            size,
            source,
            destination,
        };
        assert_eq!(
            encode(instruction, Target::H8SX, Mode::Maximum)
                .unwrap()
                .as_bytes(),
            bytes
        );
        assert_eq!(
            decode(&bytes, Target::H8SX, Mode::Maximum)
                .unwrap()
                .instruction,
            instruction
        );
        assert_eq!(disassemble(instruction), text);
    }
    for (operation, size, source, destination) in [
        (Add, Byte, 16, 0),
        (Add, Word, 0, 16),
        (Move, Long, 8, 0),
        (Move, Long, 0, 8),
        (And, Long, 0, 0),
        (AddCarry, Word, 0, 0),
        (SubtractCarry, Long, 0, 0),
    ] {
        assert!(encode(
            SxInstruction::RegisterBinary {
                operation,
                size,
                source,
                destination
            },
            Target::H8SX,
            Mode::Maximum
        )
        .is_err());
    }
    // Compact immediate forms share the long-register opcode byte, but must
    // not be mistaken for register pairs.
    for low in [0x00, 0x08, 0x78, 0x88, 0xf8] {
        assert!(decode(&[0x0a, low], Target::H8SX, Mode::Maximum).is_none());
    }
}
