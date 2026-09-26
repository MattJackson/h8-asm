//! Instruction boundaries in a big-endian H8 instruction stream.
//!
//! Length recognition covers the legacy opcode maps and H8SX §2.4 rows.
//! H8SX rules are generated from checked manual facts with explicit operand
//! constraints. Semantic decoding remains narrower than length recognition.

mod codec;
mod render;
pub use render::disassemble_insn;
pub mod decode;
pub mod insn;
mod legacy;
pub use codec::{decode_insn, encode_insn, CodecError};
pub use insn::{Decoded, Ea, Encoding, Insn, Operand, Reg, Size};
pub mod disasm;
pub mod encode;
mod length;
mod sx_codec;
pub mod sx_disasm;
pub mod sx_encode;
mod sx_operands;
mod sx_table;
#[rustfmt::skip]
mod sx_table_data;
pub(crate) use sx_table::complex_relative as sx_complex_relative;
pub mod sx_register;
pub mod sx_semantic;

use crate::{Mode, Target};

/// Maximum length, in bytes, of an H8 instruction across the supported cores.
///
/// H8SX REJ09B0102 §2.4 Table 2.2 has at most three 16-bit opcode words
/// and two 32-bit operand extensions (6 + 8 = 14 bytes). Its ADD.B
/// `@(d:32,ERs),@(d:32,ERd)` row attains this bound. Earlier cores have
/// shorter instruction-code tables (H8S REJ09B0139 §2.4 Table 2.2).
pub const MAX_INSN_LEN: usize = 14;

/// Returns the length of the first complete, recognized instruction in `bytes`.
///
/// Returns `None` for a truncated instruction, an undefined encoding, an
/// unsupported target/mode pair, or a pattern outside the implemented manual
/// row grammar. Trailing bytes are ignored. This checks encoding and
/// length, not whether executing the instruction is safe in the current state.
///
/// Unlike Thumb, H8 instruction length cannot always be determined from the
/// first word: later opcode words must be checked, sometimes after an address
/// extension. Words are read most-significant byte first. See H8/300 Appendix
/// A and §2, H8/300H §2.4, H8S §2.4 Table 2.2, and H8SX §2.4.
///
/// ```
/// use h8_asm::{isa::insn_len, Mode, Target};
/// assert_eq!(insn_len(&[0x79, 0x00, 0x12, 0x34], Target::H8_300, Mode::Normal), Some(4));
/// assert_eq!(insn_len(&[0x79, 0x00], Target::H8_300, Mode::Normal), None);
/// ```
pub fn insn_len(bytes: &[u8], target: Target, mode: Mode) -> Option<usize> {
    if !target.supports(mode) {
        return None;
    }
    let len = if target == Target::H8SX {
        usize::from(sx_table::recognize(bytes)?.len)
    } else {
        length::recognize(bytes, target)?
    };
    if bytes.len() < len {
        None
    } else {
        Some(len)
    }
}
