//! Instruction boundaries in a big-endian H8 instruction stream.
//!
//! Length recognition covers H8/300, H8/300H and H8S, plus selected H8SX
//! opcode families. Other H8SX encodings are refused until their extended
//! opcode tables are implemented.

pub mod decode;
mod length;
mod sx_length;
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
/// unsupported target/mode pair, or an unimplemented encoding (currently most
/// H8SX instructions). Trailing bytes are ignored. This checks encoding and
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
        sx_length::recognize(bytes)?
    } else {
        length::recognize(bytes, target)?
    };
    if bytes.len() < len {
        None
    } else {
        Some(len)
    }
}
