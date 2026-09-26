//! Typed interpretation of H8SX table fields.
use super::{Ea, Insn, Operand, Reg, Size};

#[derive(Clone, Copy)]
pub(super) struct Field(pub u8, pub u8, pub u8);

impl Field {
    fn read(self, input: u128) -> u32 {
        (((input >> self.0) & ((1u128 << self.1) - 1)) as u32) << self.2
    }

    fn write(self, input: &mut u128, value: u32) {
        let mask = ((1u128 << self.1) - 1) << self.0;
        *input = (*input & !mask) | ((u128::from(value >> self.2) << self.0) & mask);
    }
}

#[derive(Clone, Copy)]
pub(super) struct Register(pub Reg, pub Field);

impl Register {
    fn read(self, input: u128) -> Reg {
        let value = self.1.read(input) as u8;
        match self.0 {
            Reg::Byte(base) => Reg::Byte(base + value),
            Reg::Word(base) => Reg::Word(base + value),
            Reg::Long(base) => Reg::Long(base + value),
            other => other,
        }
    }

    fn write(self, input: &mut u128, register: Reg) -> Option<()> {
        let value = match (self.0, register) {
            (Reg::Byte(base), Reg::Byte(value))
            | (Reg::Word(base), Reg::Word(value))
            | (Reg::Long(base), Reg::Long(value)) => value.checked_sub(base)?,
            (expected, actual) if expected == actual => 0,
            _ => return None,
        };
        self.1.write(input, u32::from(value));
        Some(())
    }
}

#[derive(Clone, Copy)]
pub(super) enum AddressKind {
    Indirect,
    PostIncrement,
    PreDecrement,
    PostDecrement,
    PreIncrement,
    Displacement,
    Indexed,
    Absolute,
    PcRelative,
    PcIndexed,
    MemoryIndirect,
    ExtendedIndirect,
}

#[derive(Clone, Copy)]
pub(super) struct AddressSpec {
    pub kind: AddressKind,
    pub register: Register,
    pub value: Field,
    pub bits: u8,
}

fn signed(value: u32, bits: u8) -> i32 {
    match bits {
        8 => i32::from(value as i8),
        16 => i32::from(value as i16),
        _ => value as i32,
    }
}

impl AddressSpec {
    fn read(self, input: u128) -> Ea {
        let reg = self.register.read(input);
        let value = self.value.read(input);
        let bits = self.bits;
        match self.kind {
            AddressKind::Indirect => Ea::Indirect(reg),
            AddressKind::PostIncrement => Ea::PostIncrement(reg),
            AddressKind::PreDecrement => Ea::PreDecrement(reg),
            AddressKind::PostDecrement => Ea::PostDecrement(reg),
            AddressKind::PreIncrement => Ea::PreIncrement(reg),
            AddressKind::Displacement => Ea::Displacement {
                base: reg,
                value: signed(value, bits),
                bits,
            },
            AddressKind::Indexed => Ea::Indexed {
                index: reg,
                value: signed(value, bits),
                bits,
            },
            AddressKind::Absolute => Ea::Absolute { value, bits },
            AddressKind::PcRelative => Ea::PcRelative {
                value: signed(value, bits),
                bits,
            },
            AddressKind::PcIndexed => Ea::PcIndexed(reg),
            AddressKind::MemoryIndirect => Ea::MemoryIndirect(value as u8),
            AddressKind::ExtendedIndirect => Ea::ExtendedIndirect(value as u8),
        }
    }

    fn write(self, input: &mut u128, ea: Ea) -> Option<()> {
        // Exact kind, width, range and scaling are checked by semantic
        // decoding of the candidate after these field writes.
        match ea {
            Ea::Indirect(reg)
            | Ea::PostIncrement(reg)
            | Ea::PreDecrement(reg)
            | Ea::PostDecrement(reg)
            | Ea::PreIncrement(reg)
            | Ea::PcIndexed(reg) => self.register.write(input, reg)?,
            Ea::Displacement { base, value, .. } => {
                self.register.write(input, base)?;
                self.value.write(input, value as u32);
            }
            Ea::Indexed { index, value, .. } => {
                self.register.write(input, index)?;
                self.value.write(input, value as u32);
            }
            Ea::Absolute { value, .. } => self.value.write(input, value),
            Ea::PcRelative { value, .. } => self.value.write(input, value as u32),
            Ea::MemoryIndirect(value) | Ea::ExtendedIndirect(value) => {
                self.value.write(input, u32::from(value))
            }
        }
        Some(())
    }
}

pub(super) enum Template {
    None,
    Register(Register),
    Immediate(Field, u8),
    Constant(u32),
    Address(AddressSpec),
    Registers(Field, u8, bool),
    IndexedMemory(AddressSpec, Size, Field, u8),
}

impl Template {
    fn read(&self, input: u128) -> Operand {
        match *self {
            Self::None => Operand::None,
            Self::Register(reg) => Operand::Register(reg.read(input)),
            Self::Immediate(field, bits) => Operand::Immediate {
                value: field.read(input),
                bits,
            },
            Self::Constant(value) => Operand::Immediate { value, bits: 0 },
            Self::Address(address) => Operand::Address(address.read(input)),
            Self::Registers(field, count, last) => {
                let number = field.read(input) as u8;
                Operand::Registers {
                    first: if last { number - count } else { number },
                    last: if last { number } else { number + count },
                }
            }
            Self::IndexedMemory(address, index_size, field, bits) => Operand::IndexedMemory {
                address: address.read(input),
                index_size,
                value: signed(field.read(input), bits),
                bits,
            },
        }
    }

    fn write(&self, input: &mut u128, operand: Operand) -> Option<()> {
        match (self, operand) {
            (Self::None, Operand::None) => {}
            (Self::Register(reg), Operand::Register(value)) => reg.write(input, value)?,
            (Self::Immediate(field, _), Operand::Immediate { value, .. }) => {
                field.write(input, value)
            }
            (Self::Constant(expected), Operand::Immediate { value, bits: 0 })
                if value == *expected => {}
            (Self::Address(address), Operand::Address(value)) => address.write(input, value)?,
            (Self::Registers(field, _, last), Operand::Registers { first, last: end }) => {
                field.write(input, u32::from(if *last { end } else { first }))
            }
            (
                Self::IndexedMemory(address, _, field, _),
                Operand::IndexedMemory {
                    address: ea, value, ..
                },
            ) => {
                address.write(input, ea)?;
                field.write(input, value as u32);
            }
            _ => return None,
        }
        Some(())
    }
}

pub(super) struct Spec {
    pub mnemonic: &'static str,
    pub size: Option<Size>,
    pub operands: [Template; 3],
}

impl Spec {
    pub fn read(&self, input: u128) -> Insn {
        Insn::new(
            self.mnemonic,
            self.size,
            [
                self.operands[0].read(input),
                self.operands[1].read(input),
                self.operands[2].read(input),
            ],
        )
    }

    pub fn write(&self, input: &mut u128, insn: Insn) -> Option<()> {
        if self.mnemonic != insn.mnemonic || self.size != insn.size {
            return None;
        }
        for (template, operand) in self.operands.iter().zip(insn.operands) {
            template.write(input, operand)?;
        }
        Some(())
    }
}

pub(super) struct Family {
    pub mnemonic: &'static str,
    pub size: Option<Size>,
    pub rows: &'static [u16],
}
