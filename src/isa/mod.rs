//! Instruction boundaries in a big-endian H8 instruction stream.
//!
//! Length recognition covers H8/300, H8/300H and H8S, and a small set of
//! fixed H8SX opcodes. Other H8SX encodings are refused until their extended
//! opcode tables are implemented.

mod length;
mod sx_length;

use crate::{Mode, Target};

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
