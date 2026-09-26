//! Shared semantic decode/verified encode entry points.
use super::{
    insn::{Decoded, Encoding, Insn},
    legacy, sx_codec,
};
use crate::{Mode, Target};

/// A request outside the verified shared codec's implemented language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    /// Unsupported target/mode combination or unimplemented target.
    UnsupportedTarget,
    /// Invalid, truncated, or unimplemented instruction bytes.
    UnsupportedEncoding,
    /// The requested semantics have no exact supported encoding.
    UnsupportedOperands,
}

/// Decodes a recognized semantic instruction, ignoring trailing bytes.
/// Covers all five targets. H8SX operands come from the reviewed §2.4 table;
/// the original subset APIs remain available for existing callers.
pub fn decode_insn(bytes: &[u8], target: Target, mode: Mode) -> Result<Decoded, CodecError> {
    if !target.supports(mode) {
        return Err(CodecError::UnsupportedTarget);
    }
    if target == Target::H8SX {
        return sx_codec::decode(bytes).ok_or(CodecError::UnsupportedEncoding);
    }
    legacy::decode(bytes, target, mode).ok_or(CodecError::UnsupportedEncoding)
}

/// Encodes a shared instruction only when decoding the result reproduces every
/// semantic field and operand width. Unsupported or out-of-range operands
/// return an error; no partially encoded bytes are returned.
pub fn encode_insn(insn: Insn, target: Target, mode: Mode) -> Result<Vec<u8>, CodecError> {
    if !target.supports(mode) {
        return Err(CodecError::UnsupportedTarget);
    }
    if target == Target::H8SX {
        return sx_codec::encode(insn).ok_or(CodecError::UnsupportedOperands);
    }
    let mut bytes = legacy::encode(insn, target).ok_or(CodecError::UnsupportedOperands)?;
    if insn.encoding == Encoding::DisplacementStoreAlias {
        if bytes.len() != 10 || bytes[..3] != [1, 0, 0x78] || bytes[5] & 0x80 == 0 {
            return Err(CodecError::UnsupportedOperands);
        }
        bytes[3] |= 0x80;
    }
    if legacy::decode(&bytes, target, mode)
        != Some(Decoded {
            insn,
            len: bytes.len(),
        })
    {
        return Err(CodecError::UnsupportedOperands);
    }
    Ok(bytes)
}
