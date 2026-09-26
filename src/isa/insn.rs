//! Shared, allocation-free H8 instruction and operand vocabulary.
//!
//! Register and addressing classes follow H8/300 ADE-602-025 §1 and H8SX
//! REJ09B0102 §1.5–1.8. Width fields retain the encoded form, not just its
//! effective value, so short/long encodings can round-trip exactly.

/// Data operand size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    /// Eight bits.
    Byte,
    /// Sixteen bits.
    Word,
    /// Thirty-two bits.
    Long,
}

/// General or control register. Encoders validate register codes per target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reg {
    /// Four-bit byte code: R0H–R7H followed by R0L–R7L.
    Byte(u8),
    /// Word code: R0–R7 followed by E0–E7 on extended cores.
    Word(u8),
    /// ER0–ER7.
    Long(u8),
    /// Condition-code register.
    Ccr,
    /// Extended control register.
    Exr,
    /// High multiply-accumulate register.
    Mach,
    /// Low multiply-accumulate register.
    Macl,
    /// Vector base register (H8SX).
    Vbr,
    /// Short-address base register (H8SX).
    Sbr,
}

/// Effective-address forms. Address/displacement widths are encoded bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ea {
    /// Register indirect.
    Indirect(Reg),
    /// Register indirect with post-increment.
    PostIncrement(Reg),
    /// Register indirect with pre-decrement.
    PreDecrement(Reg),
    /// Register indirect with post-decrement (H8SX).
    PostDecrement(Reg),
    /// Register indirect with pre-increment (H8SX).
    PreIncrement(Reg),
    /// Register plus signed displacement.
    Displacement {
        /// Address register.
        base: Reg,
        /// Signed displacement.
        value: i32,
        /// Encoded displacement width.
        bits: u8,
    },
    /// Displacement plus a sized index register (H8SX).
    Indexed {
        /// Index register; its type retains the index size.
        index: Reg,
        /// Signed displacement.
        value: i32,
        /// Encoded displacement width.
        bits: u8,
    },
    /// Absolute address field, before mode-dependent extension or SBR addition.
    Absolute {
        /// Raw unsigned address field.
        value: u32,
        /// Address field width.
        bits: u8,
    },
    /// Signed offset relative to the following instruction.
    PcRelative {
        /// Signed displacement.
        value: i32,
        /// Encoded displacement width.
        bits: u8,
    },
    /// Program counter plus a sized index register (H8SX).
    PcIndexed(Reg),
    /// Memory-indirect table address in the first 256 bytes.
    MemoryIndirect(u8),
    /// Extended-memory-indirect seven-bit vector selector (H8SX).
    ExtendedIndirect(u8),
}

/// One explicit operand. Unused instruction operand slots hold `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operand {
    /// No operand in this slot.
    None,
    /// A general or control register.
    Register(Reg),
    /// Immediate bit pattern. Zero width denotes an implicit encoded count.
    Immediate {
        /// Literal value.
        value: u32,
        /// Encoded width, or zero for an implicit count.
        bits: u8,
    },
    /// An effective address or PC-relative target expression.
    Address(Ea),
    /// MOVA's displacement plus an index fetched through a memory address.
    /// The inner address uses index_size independently of the result scaling.
    IndexedMemory {
        /// Address from which the index is loaded.
        address: Ea,
        /// Size of the loaded index (byte or word).
        index_size: Size,
        /// Signed outer displacement.
        value: i32,
        /// Encoded outer displacement width.
        bits: u8,
    },
    /// An inclusive consecutive ER register range.
    Registers {
        /// First ER register number.
        first: u8,
        /// Last ER register number.
        last: u8,
    },
}

/// An architectural encoding choice not distinguished by operand semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// The normal encoding of the stated operands.
    Standard,
    /// H8S Table 2.2 note 2 permits bit 7 in the fourth byte of the
    /// displacement MOV.L store form. Preserve that bit when it was present.
    DisplacementStoreAlias,
    /// A noncanonical H8SX encoding form, identified by its row in the
    /// bundled Rev.4 instruction table. Decoders set this only when the
    /// default form would encode the same operands differently.
    SxAlternative(u16),
}

/// Shared instruction semantics, retaining operand encoding widths.
///
/// Mnemonics use the uppercase Renesas spelling without a size suffix. The
/// fixed operand array avoids allocation while walking firmware. Public values
/// can describe unsupported requests; [`super::encode_insn`] refuses them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Insn {
    /// Uppercase mnemonic (for example `MOV`, `BNE`, or `BIST`).
    pub mnemonic: &'static str,
    /// Explicit data size, if the mnemonic has one.
    pub size: Option<Size>,
    /// Operands in assembly order, followed by `None` slots.
    pub operands: [Operand; 3],
    /// Encoding choice retained for explicitly documented aliases.
    pub encoding: Encoding,
}

impl Insn {
    /// Constructs an instruction with up to three explicit operands.
    pub const fn new(mnemonic: &'static str, size: Option<Size>, operands: [Operand; 3]) -> Self {
        Self {
            mnemonic,
            size,
            operands,
            encoding: Encoding::Standard,
        }
    }
}

/// One decoded instruction and its complete byte length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Decoded {
    /// Instruction meaning.
    pub insn: Insn,
    /// Consumed byte count.
    pub len: usize,
}
