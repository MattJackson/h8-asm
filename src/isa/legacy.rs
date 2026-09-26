//! Semantic opcode map for H8/300, H8/300H and H8S.
//!
//! ADE-602-025 §2/Appendix A; REJ09B0213 §2.4; REJ09B0139 §2.4.
//! The length recognizer validates target-specific fixed bits before fields
//! are extracted. Encoding is independently dispatched and decoded for equality.

use super::{
    insn::{Decoded, Ea, Encoding, Insn, Operand, Reg, Size},
    insn_len,
};
use crate::{Mode, Target};
use Operand::{Address as A, Immediate as I, None as N, Register as R};
use Size::{Byte as B, Long as L, Word as W};

pub(super) const CONDITIONS: [&str; 16] = [
    "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
    "BLT", "BGT", "BLE",
];
const PAIRS: [(u8, &str, Size); 20] = [
    (0x08, "ADD", B),
    (0x0c, "MOV", B),
    (0x0e, "ADDX", B),
    (0x14, "OR", B),
    (0x15, "XOR", B),
    (0x16, "AND", B),
    (0x18, "SUB", B),
    (0x1c, "CMP", B),
    (0x1e, "SUBX", B),
    (0x09, "ADD", W),
    (0x0d, "MOV", W),
    (0x19, "SUB", W),
    (0x1d, "CMP", W),
    (0x64, "OR", W),
    (0x65, "XOR", W),
    (0x66, "AND", W),
    (0x0a, "ADD", L),
    (0x0f, "MOV", L),
    (0x1a, "SUB", L),
    (0x1f, "CMP", L),
];
const IMMEDIATE: [&str; 7] = ["MOV", "ADD", "CMP", "SUB", "OR", "XOR", "AND"];
const BYTE_IMMEDIATE: [&str; 8] = ["ADD", "ADDX", "CMP", "SUBX", "OR", "XOR", "AND", "MOV"];

fn insn(mnemonic: &'static str, size: Option<Size>, left: Operand, right: Operand) -> Insn {
    Insn::new(mnemonic, size, [left, right, N])
}
fn reg(size: Size, code: u8) -> Reg {
    match size {
        B => Reg::Byte(code),
        W => Reg::Word(code),
        L => Reg::Long(code),
    }
}
fn pointer(target: Target, code: u8) -> Reg {
    if target == Target::H8_300 {
        Reg::Word(code)
    } else {
        Reg::Long(code)
    }
}
fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}
fn long(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

// Internal semantic helpers consume only encodings accepted by insn_len.
fn bit(hi: u8, lo: u8, destination: Operand) -> Insn {
    let (mnemonic, source) = match hi {
        0x60..=0x63 => (
            ["BSET", "BNOT", "BCLR", "BTST"][usize::from(hi - 0x60)],
            R(Reg::Byte(lo >> 4)),
        ),
        0x67 => (
            if lo & 0x80 == 0 { "BST" } else { "BIST" },
            I {
                value: u32::from((lo >> 4) & 7),
                bits: 3,
            },
        ),
        0x70..=0x73 => (
            ["BSET", "BNOT", "BCLR", "BTST"][usize::from(hi - 0x70)],
            I {
                value: u32::from((lo >> 4) & 7),
                bits: 3,
            },
        ),
        _ => {
            // Remaining validated bit opcodes are 0x74..=0x77.
            let row = if lo & 0x80 == 0 {
                ["BOR", "BXOR", "BAND", "BLD"]
            } else {
                ["BIOR", "BIXOR", "BIAND", "BILD"]
            };
            (
                row[usize::from(hi - 0x74)],
                I {
                    value: u32::from((lo >> 4) & 7),
                    bits: 3,
                },
            )
        }
    };
    insn(mnemonic, None, source, destination)
}

fn move_memory(
    bytes: &[u8],
    target: Target,
    offset: usize,
    size: Size,
    control: Option<Reg>,
) -> Insn {
    let hi = bytes[offset];
    let lo = bytes[offset + 1];
    let store = lo & 0x80 != 0;
    let data = R(control.unwrap_or_else(|| reg(size, lo & 15)));
    let base = pointer(target, (lo >> 4) & 7);
    let ea = match hi {
        0x68 | 0x69 => Ea::Indirect(base),
        0x6c | 0x6d => {
            if store {
                Ea::PreDecrement(base)
            } else {
                Ea::PostIncrement(base)
            }
        }
        0x6e | 0x6f => Ea::Displacement {
            base,
            value: i32::from(word(bytes, offset + 2) as i16),
            bits: 16,
        },
        0x6a | 0x6b => {
            let (value, bits) = if lo & 0x20 == 0 {
                (u32::from(word(bytes, offset + 2)), 16)
            } else {
                (
                    long(bytes, offset + 2),
                    if target == Target::H8_300H { 24 } else { 32 },
                )
            };
            Ea::Absolute { value, bits }
        }
        _ => {
            // Remaining validated memory form is displacement:24/32.
            let value = long(bytes, offset + 4);
            let bits = if target == Target::H8_300H { 24 } else { 32 };
            let value = if bits == 24 {
                (value << 8) as i32 >> 8
            } else {
                value as i32
            };
            let base = pointer(target, (lo >> 4) & 7);
            let store = bytes[offset + 3] & 0x80 != 0;
            let data = R(control.unwrap_or_else(|| reg(size, bytes[offset + 3] & 15)));
            let ea = A(Ea::Displacement { base, value, bits });
            return insn(
                if control.is_some() {
                    if store {
                        "STC"
                    } else {
                        "LDC"
                    }
                } else {
                    "MOV"
                },
                Some(size),
                if store { data } else { ea },
                if store { ea } else { data },
            );
        }
    };
    let mnemonic = if control.is_some() {
        if store {
            "STC"
        } else {
            "LDC"
        }
    } else if hi == 0x6a && lo & 0x40 != 0 {
        if store {
            "MOVTPE"
        } else {
            "MOVFPE"
        }
    } else {
        "MOV"
    };
    insn(
        mnemonic,
        Some(size),
        if store { data } else { A(ea) },
        if store { A(ea) } else { data },
    )
}

fn prefix(bytes: &[u8], target: Target) -> Insn {
    let p = word(bytes, 0);
    let second = word(bytes, 2);
    match p {
        0x0100 => {
            let mut decoded = move_memory(bytes, target, 2, L, None);
            if bytes[2] == 0x78 && bytes[3] & 0x80 != 0 {
                decoded.encoding = Encoding::DisplacementStoreAlias;
            }
            decoded
        }
        0x0140 | 0x0141 => {
            let control = if p == 0x0140 { Reg::Ccr } else { Reg::Exr };
            if p == 0x0141 && matches!(bytes[2], 4..=7) {
                insn(
                    ["ORC", "XORC", "ANDC", "LDC"][usize::from(bytes[2] - 4)],
                    if bytes[2] == 7 { Some(B) } else { None },
                    I {
                        value: u32::from(bytes[3]),
                        bits: 8,
                    },
                    R(control),
                )
            } else {
                move_memory(bytes, target, 2, W, Some(control))
            }
        }
        0x0110 | 0x0120 | 0x0130 => {
            let count = ((p >> 4) & 3) as u8;
            let register = (second & 7) as u8;
            if second & 0x80 == 0 {
                insn(
                    "LDM",
                    Some(L),
                    A(Ea::PostIncrement(Reg::Long(7))),
                    Operand::Registers {
                        first: register - count,
                        last: register,
                    },
                )
            } else {
                insn(
                    "STM",
                    Some(L),
                    Operand::Registers {
                        first: register,
                        last: register + count,
                    },
                    A(Ea::PreDecrement(Reg::Long(7))),
                )
            }
        }
        0x0160 => insn(
            "MAC",
            None,
            A(Ea::PostIncrement(Reg::Long(((second >> 4) & 7) as u8))),
            A(Ea::PostIncrement(Reg::Long((second & 7) as u8))),
        ),
        0x01c0 | 0x01d0 => {
            let size = if bytes[2] < 0x52 { B } else { W };
            insn(
                if p == 0x01c0 { "MULXS" } else { "DIVXS" },
                Some(size),
                R(reg(size, bytes[3] >> 4)),
                R(reg(if size == B { W } else { L }, bytes[3] & 15)),
            )
        }
        0x01e0 => insn(
            "TAS",
            Some(B),
            A(Ea::Indirect(Reg::Long((bytes[3] >> 4) & 7))),
            N,
        ),
        _ => insn(
            ["OR", "XOR", "AND"][usize::from(bytes[2] - 0x64)],
            Some(L),
            R(Reg::Long(bytes[3] >> 4)),
            R(Reg::Long(bytes[3] & 7)),
        ),
    }
}

/// Decode one legacy instruction, retaining its exact operand widths.
pub(super) fn decode(bytes: &[u8], target: Target, mode: Mode) -> Option<Decoded> {
    let len = insn_len(bytes, target, mode)?;
    let hi = bytes[0];
    let lo = bytes[1];
    let decoded = match (hi, lo) {
        (0, 0) => insn("NOP", None, N, N),
        (1, 0x80) => insn("SLEEP", None, N, N),
        (1, 0xa0) => insn("CLRMAC", None, N, N),
        (1, _) => prefix(bytes, target),
        (0x54, 0x70) => insn("RTS", None, N, N),
        (0x56, 0x70) => insn("RTE", None, N, N),
        (2 | 3, 0..=31) => {
            let control = R(if lo & 16 == 0 { Reg::Ccr } else { Reg::Exr });
            let data = R(Reg::Byte(lo & 15));
            insn(
                if hi == 2 { "STC" } else { "LDC" },
                Some(B),
                if hi == 2 { control } else { data },
                if hi == 2 { data } else { control },
            )
        }
        (2 | 3, _) => {
            let control = R(if lo & 16 == 0 { Reg::Mach } else { Reg::Macl });
            let data = R(Reg::Long(lo & 7));
            insn(
                if hi == 2 { "STMAC" } else { "LDMAC" },
                None,
                if hi == 2 { control } else { data },
                if hi == 2 { data } else { control },
            )
        }
        (4..=7, _) => insn(
            ["ORC", "XORC", "ANDC", "LDC"][usize::from(hi - 4)],
            if hi == 7 { Some(B) } else { None },
            I {
                value: u32::from(lo),
                bits: 8,
            },
            R(Reg::Ccr),
        ),
        (0x0a | 0x1a, 0..=15) => insn(
            if hi == 0x0a { "INC" } else { "DEC" },
            Some(B),
            R(Reg::Byte(lo)),
            N,
        ),
        (0x0f | 0x1f, 0..=15) => insn(
            if hi == 0x0f { "DAA" } else { "DAS" },
            Some(B),
            R(Reg::Byte(lo)),
            N,
        ),
        (0x0b | 0x1b, _) => {
            let high = lo >> 4;
            let (mnemonic, size, count) = match high {
                0 | 8 | 9 => (
                    if hi == 0x0b { "ADDS" } else { "SUBS" },
                    if target == Target::H8_300 { W } else { L },
                    match high {
                        0 => 1,
                        8 => 2,
                        _ => 4,
                    },
                ),
                _ => (
                    if hi == 0x0b { "INC" } else { "DEC" },
                    if high & 2 == 0 { W } else { L },
                    if high & 8 == 0 { 1 } else { 2 },
                ),
            };
            insn(
                mnemonic,
                Some(size),
                I {
                    value: count,
                    bits: 0,
                },
                R(reg(size, lo & if size == L { 7 } else { 15 })),
            )
        }
        (0x10..=0x13, _) => {
            let size = match (lo >> 4) & 3 {
                0 => B,
                1 => W,
                _ => L,
            };
            let mnemonic = if lo & 0x80 == 0 {
                ["SHLL", "SHLR", "ROTXL", "ROTXR"]
            } else {
                ["SHAL", "SHAR", "ROTL", "ROTR"]
            }[usize::from(hi - 0x10)];
            insn(
                mnemonic,
                Some(size),
                I {
                    value: if lo & 0x40 == 0 { 1 } else { 2 },
                    bits: 0,
                },
                R(reg(size, lo & if size == L { 7 } else { 15 })),
            )
        }
        (0x17, _) => {
            let size = match (lo >> 4) & 3 {
                0 => B,
                1 => W,
                _ => L,
            };
            let mnemonic = match lo & 0xc0 {
                0 => "NOT",
                0x40 => "EXTU",
                0x80 => "NEG",
                _ => "EXTS",
            };
            insn(
                mnemonic,
                Some(size),
                R(reg(size, lo & if size == L { 7 } else { 15 })),
                N,
            )
        }
        (0x20..=0x3f, _) => {
            let data = R(Reg::Byte(hi & 15));
            let ea = A(Ea::Absolute {
                value: u32::from(lo),
                bits: 8,
            });
            insn(
                "MOV",
                Some(B),
                if hi < 0x30 { ea } else { data },
                if hi < 0x30 { data } else { ea },
            )
        }
        (0x40..=0x4f, _) => insn(
            CONDITIONS[usize::from(hi & 15)],
            None,
            A(Ea::PcRelative {
                value: i32::from(lo as i8),
                bits: 8,
            }),
            N,
        ),
        (0x55, _) => insn(
            "BSR",
            None,
            A(Ea::PcRelative {
                value: i32::from(lo as i8),
                bits: 8,
            }),
            N,
        ),
        (0x58 | 0x5c, _) => insn(
            if hi == 0x58 {
                CONDITIONS[usize::from(lo >> 4)]
            } else {
                "BSR"
            },
            None,
            A(Ea::PcRelative {
                value: i32::from(word(bytes, 2) as i16),
                bits: 16,
            }),
            N,
        ),
        (0x50..=0x53, _) => {
            let size = if hi < 0x52 { B } else { W };
            insn(
                if hi & 1 == 0 { "MULXU" } else { "DIVXU" },
                Some(size),
                R(reg(size, lo >> 4)),
                R(reg(if size == B { W } else { L }, lo & 15)),
            )
        }
        (0x57, _) => insn(
            "TRAPA",
            None,
            I {
                value: u32::from(lo >> 4),
                bits: 2,
            },
            N,
        ),
        (0x59 | 0x5d, _) => insn(
            if hi == 0x59 { "JMP" } else { "JSR" },
            None,
            A(Ea::Indirect(pointer(target, lo >> 4))),
            N,
        ),
        (0x5a | 0x5e, _) => insn(
            if hi == 0x5a { "JMP" } else { "JSR" },
            None,
            A(Ea::Absolute {
                value: u32::from_be_bytes([0, lo, bytes[2], bytes[3]]),
                bits: if target == Target::H8_300 { 16 } else { 24 },
            }),
            N,
        ),
        (0x5b | 0x5f, _) => insn(
            if hi == 0x5b { "JMP" } else { "JSR" },
            None,
            A(Ea::MemoryIndirect(lo)),
            N,
        ),
        (0x60..=0x63 | 0x67 | 0x70..=0x77, _) => bit(hi, lo, R(Reg::Byte(lo & 15))),
        (0x6a, low) if matches!(low & 0xf0, 0x10 | 0x30) => {
            let (value, bits, offset) = if low & 0x20 == 0 {
                (u32::from(word(bytes, 2)), 16, 4)
            } else {
                (long(bytes, 2), 32, 6)
            };
            bit(
                bytes[offset],
                bytes[offset + 1],
                A(Ea::Absolute { value, bits }),
            )
        }
        (0x68..=0x6f, _) => move_memory(bytes, target, 0, if hi & 1 == 0 { B } else { W }, None),
        (0x78, _) => move_memory(
            bytes,
            target,
            0,
            if bytes[2] & 1 == 0 { B } else { W },
            None,
        ),
        (0x79 | 0x7a, _) => {
            let size = if hi == 0x79 { W } else { L };
            let (value, bits) = if size == W {
                (u32::from(word(bytes, 2)), 16)
            } else {
                (long(bytes, 2), 32)
            };
            insn(
                IMMEDIATE[usize::from(lo >> 4)],
                Some(size),
                I { value, bits },
                R(reg(size, lo & 15)),
            )
        }
        (0x7b, _) => insn("EEPMOV", Some(if lo == 0x5c { B } else { W }), N, N),
        (0x7c..=0x7f, _) => {
            let ea = if hi < 0x7e {
                Ea::Indirect(pointer(target, lo >> 4))
            } else {
                Ea::Absolute {
                    value: u32::from(lo),
                    bits: 8,
                }
            };
            bit(bytes[2], bytes[3], A(ea))
        }
        (0x80..=0xff, _) => insn(
            BYTE_IMMEDIATE[usize::from((hi >> 4) - 8)],
            Some(B),
            I {
                value: u32::from(lo),
                bits: 8,
            },
            R(Reg::Byte(hi & 15)),
        ),
        _ => {
            let &(_, mnemonic, size) = PAIRS
                .iter()
                .find(|row| row.0 == hi)
                .expect("length-validated register pair");
            insn(
                mnemonic,
                Some(size),
                R(reg(size, (lo >> 4) & if size == L { 7 } else { 15 })),
                R(reg(size, lo & if size == L { 7 } else { 15 })),
            )
        }
    };
    Some(Decoded { insn: decoded, len })
}

fn code(register: Reg, size: Size) -> Option<u8> {
    match (register, size) {
        (Reg::Byte(n), B) | (Reg::Word(n), W) if n < 16 => Some(n),
        (Reg::Long(n), L) if n < 8 => Some(n),
        _ => None,
    }
}
fn address_code(register: Reg, target: Target) -> Option<u8> {
    let n = code(register, if target == Target::H8_300 { W } else { L })?;
    if n < 8 {
        Some(n)
    } else {
        None
    }
}
fn emit_word(mut bytes: Vec<u8>, value: u16) -> Vec<u8> {
    bytes.extend_from_slice(&value.to_be_bytes());
    bytes
}
fn emit_long(mut bytes: Vec<u8>, value: u32) -> Vec<u8> {
    bytes.extend_from_slice(&value.to_be_bytes());
    bytes
}

fn encode_bit(mnemonic: &str, source: Operand, destination: u8) -> Option<[u8; 2]> {
    let groups = ["BSET", "BNOT", "BCLR", "BTST"];
    if let Some(index) = groups.iter().position(|&name| name == mnemonic) {
        match source {
            R(Reg::Byte(register)) if register < 16 => {
                return Some([0x60 + index as u8, register << 4 | destination])
            }
            I { value, bits: 3 } if value < 8 => {
                return Some([0x70 + index as u8, (value as u8) << 4 | destination])
            }
            _ => return None,
        }
    }
    let (opcode, invert) = match mnemonic {
        "BST" => (0x67, 0),
        "BIST" => (0x67, 0x80),
        "BOR" => (0x74, 0),
        "BIOR" => (0x74, 0x80),
        "BXOR" => (0x75, 0),
        "BIXOR" => (0x75, 0x80),
        "BAND" => (0x76, 0),
        "BIAND" => (0x76, 0x80),
        "BLD" => (0x77, 0),
        "BILD" => (0x77, 0x80),
        _ => return None,
    };
    if let I { value, bits: 3 } = source {
        if value < 8 {
            return Some([opcode, invert | (value as u8) << 4 | destination]);
        }
    }
    None
}

fn encode_memory(
    mnemonic: &str,
    size: Size,
    left: Operand,
    right: Operand,
    target: Target,
) -> Option<Vec<u8>> {
    let (register, ea, store) = match (left, right) {
        (R(register), A(ea)) => (register, ea, true),
        (A(ea), R(register)) => (register, ea, false),
        _ => return None,
    };
    let data = code(register, size)?;
    let direction = if store { 0x80 } else { 0 };
    let word = if size == B { 0 } else { 1 };
    let bytes = match ea {
        Ea::Indirect(base) => vec![
            0x68 | word,
            direction | address_code(base, target)? << 4 | data,
        ],
        Ea::PostIncrement(base) if !store => {
            vec![0x6c | word, address_code(base, target)? << 4 | data]
        }
        Ea::PreDecrement(base) if store => {
            vec![0x6c | word, 0x80 | address_code(base, target)? << 4 | data]
        }
        Ea::Displacement {
            base,
            value,
            bits: 16,
        } => emit_word(
            vec![
                0x6e | word,
                direction | address_code(base, target)? << 4 | data,
            ],
            value as u16,
        ),
        Ea::Displacement {
            base,
            value,
            bits: 24 | 32,
        } => {
            let value = if target == Target::H8_300H {
                (value as u32) & 0x00ff_ffff
            } else {
                value as u32
            };
            emit_long(
                vec![
                    0x78,
                    address_code(base, target)? << 4,
                    0x6a | word,
                    0x20 | direction | data,
                ],
                value,
            )
        }
        Ea::Absolute { value, bits: 8 } if size == B => {
            vec![if store { 0x30 } else { 0x20 } | data, value as u8]
        }
        Ea::Absolute { value, bits: 16 } => emit_word(
            vec![
                0x6a | word,
                direction
                    | data
                    | if mnemonic == "MOVFPE" || mnemonic == "MOVTPE" {
                        0x40
                    } else {
                        0
                    },
            ],
            value as u16,
        ),
        Ea::Absolute {
            value,
            bits: 24 | 32,
        } => emit_long(vec![0x6a | word, 0x20 | direction | data], value),
        _ => return None,
    };
    if size == L {
        let mut prefixed = vec![1, 0];
        prefixed.extend(bytes);
        Some(prefixed)
    } else {
        Some(bytes)
    }
}

pub(super) fn encode(insn: Insn, target: Target) -> Option<Vec<u8>> {
    let Insn {
        mnemonic,
        size,
        operands: [left, right, third],
        ..
    } = insn;
    if third != N {
        return None;
    }
    if left == N && right == N {
        let bytes = match (mnemonic, size) {
            ("NOP", None) => vec![0, 0],
            ("SLEEP", None) => vec![1, 0x80],
            ("RTS", None) => vec![0x54, 0x70],
            ("RTE", None) => vec![0x56, 0x70],
            ("CLRMAC", None) => vec![1, 0xa0],
            ("EEPMOV", Some(B)) => vec![0x7b, 0x5c, 0x59, 0x8f],
            ("EEPMOV", Some(W)) => vec![0x7b, 0xd4, 0x59, 0x8f],
            _ => return None,
        };
        return Some(bytes);
    }
    if let (A(Ea::PcRelative { value, bits }), N) = (left, right) {
        let condition = CONDITIONS.iter().position(|&name| name == mnemonic);
        return match (condition, mnemonic, bits) {
            (Some(condition), _, 8) => Some(vec![0x40 | condition as u8, value as u8]),
            (Some(condition), _, 16) => {
                Some(emit_word(vec![0x58, (condition as u8) << 4], value as u16))
            }
            (None, "BSR", 8) => Some(vec![0x55, value as u8]),
            (None, "BSR", 16) => Some(emit_word(vec![0x5c, 0], value as u16)),
            _ => None,
        };
    }
    if matches!(mnemonic, "JMP" | "JSR") && right == N {
        let call = if mnemonic == "JSR" { 4 } else { 0 };
        return match left {
            A(Ea::Indirect(base)) => Some(vec![0x59 | call, address_code(base, target)? << 4]),
            A(Ea::Absolute {
                value,
                bits: 16 | 24,
            }) => {
                let [_, a, b, c] = value.to_be_bytes();
                Some(vec![0x5a | call, a, b, c])
            }
            A(Ea::MemoryIndirect(value)) => Some(vec![0x5b | call, value]),
            _ => None,
        };
    }
    if mnemonic == "TRAPA" {
        if let (I { value, bits: 2 }, N) = (left, right) {
            return Some(vec![0x57, (value as u8) << 4]);
        }
    }
    if matches!(mnemonic, "STC" | "LDC") {
        let (data, control, store) = if mnemonic == "STC" {
            (right, left, true)
        } else {
            (left, right, false)
        };
        let control = match control {
            R(Reg::Ccr) => 0,
            R(Reg::Exr) => 16,
            _ => return None,
        };
        if let R(Reg::Byte(register)) = data {
            return Some(vec![if store { 2 } else { 3 }, control | register]);
        }
        if let I { value, bits: 8 } = data {
            if !store {
                return Some(if control == 0 {
                    vec![7, value as u8]
                } else {
                    vec![1, 0x41, 7, value as u8]
                });
            }
        }
    }
    if matches!(mnemonic, "LDC" | "STC") && size == Some(W) {
        let (control, ea, store) = match (left, right) {
            (R(control), A(ea)) => (control, ea, true),
            (A(ea), R(control)) => (control, ea, false),
            _ => return None,
        };
        // The control register was validated above.
        let prefix = if control == Reg::Ccr { 0x40 } else { 0x41 };
        let placeholder = R(Reg::Word(0));
        let payload = encode_memory(
            "MOV",
            W,
            if store { placeholder } else { A(ea) },
            if store { A(ea) } else { placeholder },
            target,
        )?;
        let mut bytes = vec![1, prefix];
        bytes.extend(payload);
        return Some(bytes);
    }
    if mnemonic == "MAC" && size.is_none() {
        if let (
            A(Ea::PostIncrement(Reg::Long(source))),
            A(Ea::PostIncrement(Reg::Long(destination))),
        ) = (left, right)
        {
            return Some(vec![1, 0x60, 0x6d, source << 4 | destination]);
        }
    }
    if matches!(mnemonic, "LDM" | "STM") && size == Some(L) {
        let (first, last, load) = match (left, right) {
            (A(Ea::PostIncrement(Reg::Long(7))), Operand::Registers { first, last }) => {
                (first, last, true)
            }
            (Operand::Registers { first, last }, A(Ea::PreDecrement(Reg::Long(7)))) => {
                (first, last, false)
            }
            _ => return None,
        };
        let count = last.checked_sub(first)?;
        if !(1..=3).contains(&count) || last > 7 {
            return None;
        }
        return Some(vec![
            1,
            count << 4,
            0x6d,
            if load { 0x70 | last } else { 0xf0 | first },
        ]);
    }
    if mnemonic == "TAS" && size == Some(B) && right == N {
        if let A(Ea::Indirect(Reg::Long(register))) = left {
            return Some(vec![1, 0xe0, 0x7b, register << 4 | 0x0c]);
        }
    }
    if matches!(mnemonic, "STMAC" | "LDMAC") {
        let (data, control, store) = if mnemonic == "STMAC" {
            (right, left, true)
        } else {
            (left, right, false)
        };
        if let (R(Reg::Long(register)), R(control)) = (data, control) {
            let control = match control {
                Reg::Mach => 0x20,
                Reg::Macl => 0x30,
                _ => return None,
            };
            return Some(vec![if store { 2 } else { 3 }, control | register]);
        }
    }
    if matches!(mnemonic, "ORC" | "XORC" | "ANDC") {
        if let (I { value, bits: 8 }, R(control)) = (left, right) {
            let opcode = match mnemonic {
                "ORC" => 4,
                "XORC" => 5,
                _ => 6,
            };
            return match control {
                Reg::Ccr => Some(vec![opcode, value as u8]),
                Reg::Exr => Some(vec![1, 0x41, opcode, value as u8]),
                _ => None,
            };
        }
    }
    if let Some(size) = size {
        if let (R(source), R(destination)) = (left, right) {
            if size == L && matches!(mnemonic, "OR" | "XOR" | "AND") {
                let opcode = match mnemonic {
                    "OR" => 0x64,
                    "XOR" => 0x65,
                    _ => 0x66,
                };
                return Some(vec![
                    1,
                    0xf0,
                    opcode,
                    code(source, L)? << 4 | code(destination, L)?,
                ]);
            }
            if matches!(mnemonic, "MULXS" | "DIVXS") && size != L {
                return Some(vec![
                    1,
                    if mnemonic == "MULXS" { 0xc0 } else { 0xd0 },
                    if size == B { 0x50 } else { 0x52 } | if mnemonic == "DIVXS" { 1 } else { 0 },
                    code(source, size)? << 4 | code(destination, if size == B { W } else { L })?,
                ]);
            }
            if let Some(&(opcode, _, _)) =
                PAIRS.iter().find(|row| row.1 == mnemonic && row.2 == size)
            {
                return Some(vec![
                    opcode,
                    code(source, size)? << 4
                        | code(destination, size)?
                        | if size == L { 0x80 } else { 0 },
                ]);
            }
            if matches!(mnemonic, "MULXU" | "DIVXU") && size != L {
                let opcode =
                    if size == B { 0x50 } else { 0x52 } | if mnemonic == "DIVXU" { 1 } else { 0 };
                return Some(vec![
                    opcode,
                    code(source, size)? << 4 | code(destination, if size == B { W } else { L })?,
                ]);
            }
        }
        if let (I { value, bits }, R(destination)) = (left, right) {
            if size == B && bits == 8 {
                if let Some(index) = BYTE_IMMEDIATE.iter().position(|&name| name == mnemonic) {
                    return Some(vec![
                        0x80 | (index as u8) << 4 | code(destination, B)?,
                        value as u8,
                    ]);
                }
            }
            if (size == W && bits == 16) || (size == L && bits == 32) {
                if let Some(index) = IMMEDIATE.iter().position(|&name| name == mnemonic) {
                    let bytes = vec![
                        if size == W { 0x79 } else { 0x7a },
                        (index as u8) << 4 | code(destination, size)?,
                    ];
                    return Some(if size == W {
                        emit_word(bytes, value as u16)
                    } else {
                        emit_long(bytes, value)
                    });
                }
            }
            if bits == 0 {
                let register = code(destination, size)?;
                if matches!(mnemonic, "ADDS" | "SUBS") {
                    let count = match value {
                        1 => 0,
                        2 => 0x80,
                        4 => 0x90,
                        _ => return None,
                    };
                    return Some(vec![
                        if mnemonic == "ADDS" { 0x0b } else { 0x1b },
                        count | register,
                    ]);
                }
                if matches!(mnemonic, "INC" | "DEC") && size != B {
                    let count = match value {
                        1 => 0,
                        2 => 0x80,
                        _ => return None,
                    };
                    return Some(vec![
                        if mnemonic == "INC" { 0x0b } else { 0x1b },
                        count | if size == W { 0x50 } else { 0x70 } | register,
                    ]);
                }
                let (opcode, arithmetic) = match mnemonic {
                    "SHLL" => (0x10, 0),
                    "SHAL" => (0x10, 0x80),
                    "SHLR" => (0x11, 0),
                    "SHAR" => (0x11, 0x80),
                    "ROTXL" => (0x12, 0),
                    "ROTL" => (0x12, 0x80),
                    "ROTXR" => (0x13, 0),
                    "ROTR" => (0x13, 0x80),
                    _ => return None,
                };
                let count = match value {
                    1 => 0,
                    2 => 0x40,
                    _ => return None,
                };
                return Some(vec![
                    opcode,
                    arithmetic
                        | count
                        | match size {
                            B => 0,
                            W => 0x10,
                            L => 0x30,
                        }
                        | register,
                ]);
            }
        }
        if let (R(register), N) = (left, right) {
            let register = code(register, size)?;
            match mnemonic {
                "INC" | "DEC" if size == B => {
                    return Some(vec![if mnemonic == "INC" { 0x0a } else { 0x1a }, register])
                }
                "DAA" | "DAS" if size == B => {
                    return Some(vec![if mnemonic == "DAA" { 0x0f } else { 0x1f }, register])
                }
                "NOT" | "NEG" | "EXTU" | "EXTS" => {
                    let op = match mnemonic {
                        "NOT" => 0,
                        "EXTU" => 0x40,
                        "NEG" => 0x80,
                        _ => 0xc0,
                    };
                    return Some(vec![
                        0x17,
                        op | match size {
                            B => 0,
                            W => 0x10,
                            L => 0x30,
                        } | register,
                    ]);
                }
                _ => {}
            }
        }
        if matches!(mnemonic, "MOV" | "MOVFPE" | "MOVTPE") {
            return encode_memory(mnemonic, size, left, right, target);
        }
    }
    if size.is_none() {
        match right {
            R(Reg::Byte(register)) if register < 16 => {
                return encode_bit(mnemonic, left, register).map(Vec::from)
            }
            A(ea) => {
                let [hi, lo] = encode_bit(mnemonic, left, 0)?;
                let write = matches!(mnemonic, "BSET" | "BNOT" | "BCLR" | "BST" | "BIST");
                let prefix = match ea {
                    Ea::Indirect(base) => [
                        if write { 0x7d } else { 0x7c },
                        address_code(base, target)? << 4,
                    ],
                    Ea::Absolute { value, bits: 8 } => {
                        [if write { 0x7f } else { 0x7e }, value as u8]
                    }
                    Ea::Absolute { value, bits } if matches!(bits, 16 | 32) => {
                        let bytes = vec![
                            0x6a,
                            0x10 | if write { 8 } else { 0 } | if bits == 32 { 0x20 } else { 0 },
                        ];
                        let mut bytes = if bits == 16 {
                            emit_word(bytes, value as u16)
                        } else {
                            emit_long(bytes, value)
                        };
                        bytes.extend_from_slice(&[hi, lo]);
                        return Some(bytes);
                    }
                    _ => return None,
                };
                return Some(vec![prefix[0], prefix[1], hi, lo]);
            }
            _ => {}
        }
    }
    None
}
