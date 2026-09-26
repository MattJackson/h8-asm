//! Adversarial public-API inputs: accepted encodings must preserve every field.
use h8_asm::{
    isa::{decode_insn, disassemble_insn, encode_insn, Ea, Encoding, Insn, Operand, Reg, Size},
    Mode, Target,
};

#[test]
fn typed_operand_boundaries_never_silently_truncate_or_change_semantics() {
    use Operand::{Address as A, Immediate as I, None as N, Register as R};
    let mut operands = vec![N];
    for n in [0, 7, 8, 15, 16, 255] {
        operands.extend([R(Reg::Byte(n)), R(Reg::Word(n)), R(Reg::Long(n))]);
    }
    for r in [Reg::Ccr, Reg::Exr, Reg::Mach, Reg::Macl, Reg::Vbr, Reg::Sbr] {
        operands.push(R(r));
    }
    for bits in [0, 2, 3, 8, 16, 32] {
        for value in [0, 1, 2, 3, 4, 7, 8, 255, 256, 65535, 65536, u32::MAX] {
            operands.push(I { value, bits });
        }
    }
    for base in [
        Reg::Word(0),
        Reg::Word(8),
        Reg::Long(0),
        Reg::Long(7),
        Reg::Long(8),
    ] {
        operands.extend([
            A(Ea::Indirect(base)),
            A(Ea::PostIncrement(base)),
            A(Ea::PreDecrement(base)),
        ]);
        for bits in [8, 16, 24, 32] {
            for value in [-32769, -1, 32768] {
                operands.push(A(Ea::Displacement { base, value, bits }));
            }
        }
    }
    for bits in [8, 16, 24, 32] {
        for value in [0, 256, 65536, u32::MAX] {
            operands.push(A(Ea::Absolute { value, bits }));
        }
        for value in [-32769, -1, 0, 128, 32768] {
            operands.push(A(Ea::PcRelative { value, bits }));
        }
    }
    operands.extend([A(Ea::MemoryIndirect(255)), A(Ea::PcIndexed(Reg::Long(0)))]);
    for (first, last) in [(0, 1), (1, 2), (0, 3), (4, 7), (0, 0), (2, 1), (0, 8)] {
        operands.push(Operand::Registers { first, last });
    }
    let names = [
        "UNKNOWN", "NOP", "MOV", "MOVFPE", "MOVTPE", "ADD", "SUB", "CMP", "AND", "OR", "XOR",
        "ADDS", "SUBS", "INC", "DEC", "SHLL", "SHLR", "SHAL", "SHAR", "ROTL", "ROTR", "ROTXL",
        "ROTXR", "NOT", "NEG", "EXTU", "EXTS", "DAA", "DAS", "MULXU", "DIVXU", "MULXS", "DIVXS",
        "LDC", "STC", "ORC", "XORC", "ANDC", "LDMAC", "STMAC", "MAC", "LDM", "STM", "TAS", "TRAPA",
        "JMP", "JSR", "BRA", "BSR", "BSET", "BNOT", "BCLR", "BTST", "BST", "BIST", "BOR", "BIOR",
        "BXOR", "BIXOR", "BAND", "BIAND", "BLD", "BILD",
    ];
    // Extended core exercises every shared operand class; the target/mode
    // matrix and restricted encodings are independently checked elsewhere.
    let target = Target::H8S2600;
    let mode = Mode::Advanced;
    let mut accepted = 0usize;
    for name in names {
        for size in [None, Some(Size::Byte), Some(Size::Word), Some(Size::Long)] {
            for &left in &operands {
                for &right in &operands {
                    let insn = Insn::new(name, size, [left, right, N]);
                    if let Ok(bytes) = encode_insn(insn, target, mode) {
                        assert_eq!(decode_insn(&bytes, target, mode).unwrap().insn, insn);
                        assert!(!disassemble_insn(insn, target, mode).unwrap().is_empty());
                        accepted += 1;
                    }
                }
            }
        }
    }
    assert!(accepted > 1000);
    assert!(encode_insn(
        Insn::new("NOP", None, [N; 3]),
        Target::H8_300,
        Mode::Maximum
    )
    .is_err());
    let mut invalid = Insn::new("NOP", None, [N, N, R(Reg::Byte(0))]);
    assert!(encode_insn(invalid, target, mode).is_err());
    invalid.operands = [N; 3];
    invalid.encoding = Encoding::DisplacementStoreAlias;
    assert!(encode_insn(invalid, target, mode).is_err());
    assert!(disassemble_insn(invalid, target, mode).is_err());
}

#[test]
fn base_core_rejects_upper_word_register_as_address() {
    let invalid = Insn::new(
        "JMP",
        None,
        [
            Operand::Address(Ea::Indirect(Reg::Word(8))),
            Operand::None,
            Operand::None,
        ],
    );
    assert!(encode_insn(invalid, Target::H8_300, Mode::Normal).is_err());
}
