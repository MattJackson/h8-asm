use h8_asm::{
    isa::{
        insn_len,
        sx_disasm::disassemble,
        sx_encode::{encode, SxEncodeError},
        sx_semantic::{decode, SxInstruction},
    },
    Mode, Target,
};

// REJ09B0102 §2.4 immediate-register rows. Constructors deliberately use
// public variants; expected opcode bytes do not come from production tables.
type Constructor = fn(u8, u32) -> SxInstruction;

#[test]
fn immediate_register_boundaries_and_truncation() {
    let cases: &[(Constructor, u8, u8, u8, &str)] = &[
        (
            |register, immediate| SxInstruction::AddByteImmediate {
                register,
                immediate: immediate as u8,
            },
            8,
            0x80,
            0x00,
            "ADD",
        ),
        (
            |register, immediate| SxInstruction::AddCarryByteImmediate {
                register,
                immediate: immediate as u8,
            },
            8,
            0x90,
            0x00,
            "ADDX",
        ),
        (
            |register, immediate| SxInstruction::SubtractCarryByteImmediate {
                register,
                immediate: immediate as u8,
            },
            8,
            0xb0,
            0x00,
            "SUBX",
        ),
        (
            |register, immediate| SxInstruction::OrByteImmediate {
                register,
                immediate: immediate as u8,
            },
            8,
            0xc0,
            0x00,
            "OR",
        ),
        (
            |register, immediate| SxInstruction::XorByteImmediate {
                register,
                immediate: immediate as u8,
            },
            8,
            0xd0,
            0x00,
            "XOR",
        ),
        (
            |register, immediate| SxInstruction::AndByteImmediate {
                register,
                immediate: immediate as u8,
            },
            8,
            0xe0,
            0x00,
            "AND",
        ),
        (
            |register, immediate| SxInstruction::MoveWordImmediate {
                register,
                immediate: immediate as u16,
            },
            16,
            0x79,
            0x00,
            "MOV",
        ),
        (
            |register, immediate| SxInstruction::MoveLongImmediate {
                register,
                immediate,
            },
            32,
            0x7a,
            0x00,
            "MOV",
        ),
        (
            |register, immediate| SxInstruction::AddWordImmediate {
                register,
                immediate: immediate as u16,
            },
            16,
            0x79,
            0x10,
            "ADD",
        ),
        (
            |register, immediate| SxInstruction::AddLongImmediate {
                register,
                immediate,
            },
            32,
            0x7a,
            0x10,
            "ADD",
        ),
        (
            |register, immediate| SxInstruction::CompareWordImmediate {
                register,
                immediate: immediate as u16,
            },
            16,
            0x79,
            0x20,
            "CMP",
        ),
        (
            |register, immediate| SxInstruction::CompareLongImmediate {
                register,
                immediate,
            },
            32,
            0x7a,
            0x20,
            "CMP",
        ),
        (
            |register, immediate| SxInstruction::SubtractWordImmediate {
                register,
                immediate: immediate as u16,
            },
            16,
            0x79,
            0x30,
            "SUB",
        ),
        (
            |register, immediate| SxInstruction::SubtractLongImmediate {
                register,
                immediate,
            },
            32,
            0x7a,
            0x30,
            "SUB",
        ),
        (
            |register, immediate| SxInstruction::OrWordImmediate {
                register,
                immediate: immediate as u16,
            },
            16,
            0x79,
            0x40,
            "OR",
        ),
        (
            |register, immediate| SxInstruction::OrLongImmediate {
                register,
                immediate,
            },
            32,
            0x7a,
            0x40,
            "OR",
        ),
        (
            |register, immediate| SxInstruction::XorWordImmediate {
                register,
                immediate: immediate as u16,
            },
            16,
            0x79,
            0x50,
            "XOR",
        ),
        (
            |register, immediate| SxInstruction::XorLongImmediate {
                register,
                immediate,
            },
            32,
            0x7a,
            0x50,
            "XOR",
        ),
        (
            |register, immediate| SxInstruction::AndWordImmediate {
                register,
                immediate: immediate as u16,
            },
            16,
            0x79,
            0x60,
            "AND",
        ),
        (
            |register, immediate| SxInstruction::AndLongImmediate {
                register,
                immediate,
            },
            32,
            0x7a,
            0x60,
            "AND",
        ),
    ];
    for &(construct, bits, high, low, mnemonic) in cases {
        let max_register = if bits == 32 { 7 } else { 15 };
        let max_literal = (u32::MAX as u64 >> (32 - bits)) as u32;
        for register in 0..=max_register {
            for immediate in [0, 1, max_literal / 2, max_literal / 2 + 1, max_literal] {
                let instruction = construct(register, immediate);
                let mut expected = if bits == 8 {
                    vec![high | register, immediate as u8]
                } else {
                    vec![high, low | register]
                };
                if bits > 8 {
                    expected
                        .extend_from_slice(&immediate.to_be_bytes()[4 - usize::from(bits / 8)..]);
                }
                for mode in [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum] {
                    assert_eq!(
                        insn_len(&expected, Target::H8SX, mode),
                        Some(expected.len())
                    );
                    assert_eq!(
                        decode(&expected, Target::H8SX, mode).unwrap().instruction,
                        instruction
                    );
                    assert_eq!(
                        encode(instruction, Target::H8SX, mode).unwrap().as_bytes(),
                        expected
                    );
                    for end in 0..expected.len() {
                        assert!(decode(&expected[..end], Target::H8SX, mode).is_none());
                    }
                }
                let (suffix, name) = match bits {
                    8 if register < 8 => ("B", format!("R{register}H")),
                    8 => ("B", format!("R{}L", register - 8)),
                    16 if register < 8 => ("W", format!("R{register}")),
                    16 => ("W", format!("E{}", register - 8)),
                    _ => ("L", format!("ER{register}")),
                };
                let width = usize::from(bits / 4);
                assert_eq!(
                    disassemble(instruction),
                    format!("{mnemonic}.{suffix} #H'{immediate:0width$X}:{bits},{name}")
                );
            }
        }
        assert_eq!(
            encode(construct(max_register + 1, 0), Target::H8SX, Mode::Maximum),
            Err(SxEncodeError::OperandOutOfRange)
        );
    }
}
