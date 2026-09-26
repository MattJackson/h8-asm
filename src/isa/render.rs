//! Renesas syntax for the shared instruction vocabulary (REJ10J2039 §11).
use super::{encode_insn, CodecError, Ea, Insn, Operand, Reg, Size};
use crate::{Mode, Target};
use std::fmt;

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Reg::Byte(n) if n < 8 => write!(f, "R{n}H"),
            Reg::Byte(n) => write!(f, "R{}L", n.saturating_sub(8)),
            Reg::Word(n) if n < 8 => write!(f, "R{n}"),
            Reg::Word(n) => write!(f, "E{}", n.saturating_sub(8)),
            Reg::Long(n) => write!(f, "ER{n}"),
            Reg::Ccr => f.write_str("CCR"),
            Reg::Exr => f.write_str("EXR"),
            Reg::Mach => f.write_str("MACH"),
            Reg::Macl => f.write_str("MACL"),
            Reg::Vbr => f.write_str("VBR"),
            Reg::Sbr => f.write_str("SBR"),
        }
    }
}

fn sized_index(register: Reg) -> String {
    let suffix = match register {
        Reg::Byte(_) => "B",
        Reg::Word(_) => "W",
        _ => "L",
    };
    format!("{register}.{suffix}")
}

fn address(ea: Ea, len: usize, mode: Mode) -> String {
    match ea {
        Ea::Indirect(register) => format!("@{register}"),
        Ea::PostIncrement(register) => format!("@{register}+"),
        Ea::PreDecrement(register) => format!("@-{register}"),
        Ea::PostDecrement(register) => format!("@{register}-"),
        Ea::PreIncrement(register) => format!("@+{register}"),
        Ea::Displacement {
            base,
            value,
            bits: 24,
        } => format!("@(H'{:X}:24,{base})", (value as u32) & 0x00ff_ffff),
        Ea::Displacement { base, value, bits } => format!("@({value}:{bits},{base})"),
        Ea::Indexed { index, value, bits } => format!("@({value}:{bits},{})", sized_index(index)),
        Ea::Absolute { value, bits } => format!("@H'{value:X}:{bits}"),
        Ea::PcRelative { value, bits } => {
            let delta = i64::from(value) + len as i64;
            if delta >= 0 {
                format!("$+{delta}:{bits}")
            } else {
                format!("${delta}:{bits}")
            }
        }
        Ea::PcIndexed(register) => sized_index(register),
        Ea::MemoryIndirect(value) => format!("@@H'{value:X}:8"),
        Ea::ExtendedIndirect(value) => {
            let address = (u32::from(value) + 128) * if mode == Mode::Normal { 2 } else { 4 };
            format!("@@H'{address:X}:7")
        }
    }
}

/// Renders a validated shared instruction as one Renesas assembly line.
/// Encoding widths are explicit where the assembler could otherwise choose a
/// shorter form. Relative targets use `$` plus the encoded instruction length.
/// The encoder is consulted for length and operand validity before rendering.
pub fn disassemble_insn(insn: Insn, target: Target, mode: Mode) -> Result<String, CodecError> {
    let bytes = encode_insn(insn, target, mode)?;
    Ok(render(insn, bytes.len(), mode))
}

fn size_suffix(size: Size) -> &'static str {
    match size {
        Size::Byte => ".B",
        Size::Word => ".W",
        Size::Long => ".L",
    }
}

pub(super) fn render(insn: Insn, len: usize, mode: Mode) -> String {
    let mut text = insn.mnemonic.to_owned();
    if let Some(size) = insn
        .size
        .filter(|_| !matches!(insn.mnemonic, "ADDS" | "SUBS"))
    {
        text.push_str(size_suffix(size));
    }
    let mut first = true;
    for operand in insn.operands {
        let rendered = match operand {
            Operand::None => continue,
            Operand::Register(register) => register.to_string(),
            Operand::Immediate { value, bits } => {
                // A unit shift/rotate count is implicit in Renesas syntax.
                if bits == 0
                    && value == 1
                    && matches!(
                        insn.mnemonic,
                        "SHLL" | "SHLR" | "SHAL" | "SHAR" | "ROTL" | "ROTR" | "ROTXL" | "ROTXR"
                    )
                {
                    continue;
                }
                if bits == 0 {
                    format!("#{value}")
                } else if bits < 8 {
                    format!("#{value}:{bits}")
                } else {
                    format!("#H'{value:X}:{bits}")
                }
            }
            Operand::Address(ea) => address(ea, len, mode),
            Operand::IndexedMemory {
                address: ea,
                index_size,
                value,
                bits,
            } => {
                let suffix = size_suffix(index_size);
                format!("@({value}:{bits},{}{suffix})", address(ea, len, mode))
            }
            Operand::Registers { first, last } => format!("(ER{first}-ER{last})"),
        };
        text.push(if first { ' ' } else { ',' });
        first = false;
        text.push_str(&rendered);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_rendering_covers_both_operand_directions_and_widths() {
        for (bytes, target, expected) in [
            (&[0x08, 0x18][..], Target::H8_300, "ADD.B R1H,R0L"),
            (&[0x09, 0xf8][..], Target::H8_300H, "ADD.W E7,E0"),
            (
                &[0x78, 0, 0x6a, 0x20, 0, 0xff, 0xff, 0xff][..],
                Target::H8_300H,
                "MOV.B @(H'FFFFFF:24,ER0),R0H",
            ),
            (
                &[0x6e, 0x08, 0xff, 0xfc][..],
                Target::H8_300H,
                "MOV.B @(-4:16,ER0),R0L",
            ),
        ] {
            let mode = if target == Target::H8_300 {
                Mode::Normal
            } else {
                Mode::Advanced
            };
            let decoded = super::super::decode_insn(bytes, target, mode).unwrap();
            assert_eq!(
                disassemble_insn(decoded.insn, target, mode).unwrap(),
                expected
            );
        }
        for (ea, expected) in [
            (Ea::Indirect(Reg::Long(0)), "@ER0"),
            (Ea::PostIncrement(Reg::Long(0)), "@ER0+"),
            (Ea::PreDecrement(Reg::Long(0)), "@-ER0"),
            (
                Ea::Absolute {
                    value: 0xffff,
                    bits: 16,
                },
                "@H'FFFF:16",
            ),
            (Ea::PcRelative { value: -4, bits: 8 }, "$-2:8"),
            (Ea::PcRelative { value: 0, bits: 8 }, "$+2:8"),
            (Ea::MemoryIndirect(255), "@@H'FF:8"),
        ] {
            assert_eq!(address(ea, 2, Mode::Advanced), expected);
        }
        for (reg, expected) in [
            (Reg::Ccr, "CCR"),
            (Reg::Exr, "EXR"),
            (Reg::Mach, "MACH"),
            (Reg::Macl, "MACL"),
        ] {
            assert_eq!(reg.to_string(), expected);
        }
    }

    #[test]
    fn extended_vocabulary_renders_without_losing_fields() {
        let cases = [
            (Ea::PostDecrement(Reg::Long(1)), "@ER1-"),
            (Ea::PreIncrement(Reg::Long(1)), "@+ER1"),
            (
                Ea::Indexed {
                    index: Reg::Byte(8),
                    value: -4,
                    bits: 16,
                },
                "@(-4:16,R0L.B)",
            ),
            (
                Ea::Indexed {
                    index: Reg::Word(1),
                    value: 0,
                    bits: 32,
                },
                "@(0:32,R1.W)",
            ),
            (Ea::PcIndexed(Reg::Long(2)), "ER2.L"),
            (Ea::ExtendedIndirect(127), "@@H'3FC:7"),
        ];
        for (ea, want) in cases {
            assert_eq!(address(ea, 2, Mode::Maximum), want);
        }
        assert_eq!(
            address(Ea::ExtendedIndirect(0), 2, Mode::Normal),
            "@@H'100:7"
        );
        assert_eq!(Reg::Vbr.to_string(), "VBR");
        assert_eq!(Reg::Sbr.to_string(), "SBR");
    }
}
