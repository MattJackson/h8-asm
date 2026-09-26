//! Exhaustive operand-field audits, separate from the fast boundary suite.
//! Run: cargo test --release --test sx_operand_audit -- --ignored --nocapture
//! These establish internal consistency, not independent conformance.

use h8_asm::{
    isa::{
        insn_len,
        sx_encode::encode,
        sx_semantic::{decode, SxInstruction},
    },
    Mode, Target,
};

type WordConstructor = fn(u8, u16) -> SxInstruction;

#[test]
#[ignore = "exhaustive 7,340,032-pattern operand audit; run in release mode"]
fn all_word_immediate_fields() {
    let operations: [WordConstructor; 7] = [
        |register, immediate| SxInstruction::MoveWordImmediate {
            register,
            immediate,
        },
        |register, immediate| SxInstruction::AddWordImmediate {
            register,
            immediate,
        },
        |register, immediate| SxInstruction::CompareWordImmediate {
            register,
            immediate,
        },
        |register, immediate| SxInstruction::SubtractWordImmediate {
            register,
            immediate,
        },
        |register, immediate| SxInstruction::OrWordImmediate {
            register,
            immediate,
        },
        |register, immediate| SxInstruction::XorWordImmediate {
            register,
            immediate,
        },
        |register, immediate| SxInstruction::AndWordImmediate {
            register,
            immediate,
        },
    ];
    let mut count = 0u64;
    for (operation, construct) in operations.iter().enumerate() {
        for register in 0..16u8 {
            for immediate in 0..=u16::MAX {
                let [hi, lo] = immediate.to_be_bytes();
                let bytes = [0x79, (operation as u8) << 4 | register, hi, lo];
                let expected = construct(register, immediate);
                let decoded = decode(&bytes, Target::H8SX, Mode::Maximum).unwrap();
                assert_eq!(decoded.instruction, expected);
                assert_eq!(decoded.len, 4);
                assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Maximum), Some(4));
                assert_eq!(
                    encode(expected, Target::H8SX, Mode::Maximum)
                        .unwrap()
                        .as_bytes(),
                    bytes
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 7_340_032);
    println!("{count} word-immediate patterns matched typed decode, length and verified encode");
}

#[test]
#[ignore = "exhaustive 4,456,448-case branch audit; run in release mode"]
fn all_sixteen_bit_branch_fields_and_modes() {
    let mut count = 0u64;
    for mode in [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum] {
        for condition in 0..17u8 {
            for raw in 0..=u16::MAX {
                let [hi, lo] = raw.to_be_bytes();
                let bytes = if condition < 16 {
                    [0x58, condition << 4, hi, lo]
                } else {
                    [0x5c, 0, hi, lo]
                };
                let instruction = if condition < 16 {
                    SxInstruction::Branch16 {
                        condition,
                        displacement: raw as i16,
                    }
                } else {
                    SxInstruction::BranchSubroutine16 {
                        displacement: raw as i16,
                    }
                };
                if raw & 1 != 0 {
                    assert!(decode(&bytes, Target::H8SX, mode).is_none());
                    assert_eq!(insn_len(&bytes, Target::H8SX, mode), None);
                    assert!(encode(instruction, Target::H8SX, mode).is_err());
                } else {
                    let decoded = decode(&bytes, Target::H8SX, mode).unwrap();
                    assert_eq!(decoded.instruction, instruction);
                    assert_eq!(decoded.len, 4);
                    assert_eq!(insn_len(&bytes, Target::H8SX, mode), Some(4));
                    assert_eq!(
                        encode(instruction, Target::H8SX, mode).unwrap().as_bytes(),
                        bytes
                    );
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 4_456_448);
    println!("{count} signed branch field/mode cases matched, including odd-displacement refusal");
}
