//! Relocation of recognized instruction streams in flat images.
//!
//! Relative operands follow H8/300 §2 Bcc/BSR and H8SX REJ09B0102 §1.8.8–9,
//! §2.4. H8SX BRA/S (§2.2.24) and PC-indexed branches are refused. Ordinary
//! recognized instructions have no PC-relative data operands (§1.8) and retain
//! their exact bytes. This module does not infer entry points or code from data.

use crate::{isa::insn_len, Mode, Target};

/// A refusal while laying out or relocating instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelocateError {
    /// The requested CPU mode does not exist on the target.
    UnsupportedMode,
    /// A code address is odd or outside the mode's program address space.
    InvalidAddress,
    /// An instruction is unknown, invalid, or truncated at this input offset.
    UnsupportedInstruction(usize),
    /// An H8SX delayed or PC-indexed branch cannot be relocated statically.
    DynamicBranch(usize),
    /// An internal branch enters the middle of an instruction.
    InteriorBranch(u32),
    /// An absolute jump/call targets the block being moved; retargeting this
    /// form is not implemented, so copying it could re-enter overwritten code.
    AbsoluteInternalBranch(u32),
    /// A branch cannot reach its destination in the available formats.
    BranchOutOfRange,
    /// The output would cross the end of the program address space.
    AddressOverflow,
}

/// Relocated bytes and a map from each original instruction to its new offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocated {
    /// Complete relocated instruction bytes.
    pub bytes: Vec<u8>,
    /// `(original byte offset, relocated byte offset)` for each instruction.
    pub offsets: Vec<(usize, usize)>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Branch {
    Conditional(u8),
    Subroutine,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Destination {
    Address(u32),
    Node(usize),
}

pub(crate) struct Node {
    pub bytes: Vec<u8>,
    pub branch: Option<(Branch, Destination)>,
}

pub(crate) fn limit(mode: Mode) -> u64 {
    match mode {
        Mode::Normal => 1 << 16,
        Mode::Middle | Mode::Advanced => 1 << 24,
        Mode::Maximum => 1 << 32,
    }
}

pub(crate) fn address(address: u32, mode: Mode) -> Result<(), RelocateError> {
    if address & 1 != 0 || u64::from(address) >= limit(mode) {
        Err(RelocateError::InvalidAddress)
    } else {
        Ok(())
    }
}

pub(crate) fn classify(
    bytes: &[u8],
    target: Target,
    offset: usize,
) -> Result<Option<(Branch, i32)>, RelocateError> {
    let hi = bytes[0];
    let lo = bytes[1];
    if target == Target::H8SX
        && (crate::isa::sx_complex_relative(bytes)
            || hi == 0x40 && lo & 1 != 0
            || matches!(hi, 0x59 | 0x5d) && matches!(lo & 0x8f, 5..=7))
    {
        return Err(RelocateError::DynamicBranch(offset));
    }
    let branch = match hi {
        0x40..=0x4f => Some((Branch::Conditional(hi & 15), i32::from(lo as i8))),
        0x55 => Some((Branch::Subroutine, i32::from(lo as i8))),
        0x58 => Some((
            Branch::Conditional(lo >> 4),
            i32::from(i16::from_be_bytes([bytes[2], bytes[3]])),
        )),
        0x5c => Some((
            Branch::Subroutine,
            i32::from(i16::from_be_bytes([bytes[2], bytes[3]])),
        )),
        _ => None,
    };
    Ok(branch)
}

fn displacement(next: u64, destination: u32, mode: Mode) -> i64 {
    let modulus = limit(mode) as i64;
    let forward = (i64::from(destination) - next as i64).rem_euclid(modulus);
    if forward >= modulus / 2 {
        forward - modulus
    } else {
        forward
    }
}

pub(crate) fn layout(
    mut nodes: Vec<Node>,
    origin: u32,
    target: Target,
    mode: Mode,
) -> Result<(Vec<u8>, Vec<usize>), RelocateError> {
    address(origin, mode)?;
    loop {
        let mut offsets = Vec::with_capacity(nodes.len() + 1);
        offsets.push(0usize);
        for node in &nodes {
            offsets.push(offsets.last().unwrap().saturating_add(node.bytes.len()));
        }
        let end = u64::from(origin).saturating_add(*offsets.last().unwrap() as u64);
        if end > limit(mode) {
            return Err(RelocateError::AddressOverflow);
        }
        let mut widened = false;
        let mut out_of_range = false;
        for (index, node) in nodes.iter_mut().enumerate() {
            if let Some((branch, destination)) = node.branch {
                let destination = match destination {
                    Destination::Address(value) => value,
                    Destination::Node(index) => {
                        ((u64::from(origin) + offsets[index] as u64) % limit(mode)) as u32
                    }
                };
                let next = u64::from(origin) + offsets[index] as u64 + node.bytes.len() as u64;
                let delta = displacement(next, destination, mode);
                if node.bytes.len() == 2 && !(-128..=126).contains(&delta) {
                    if target == Target::H8_300 {
                        return Err(RelocateError::BranchOutOfRange);
                    }
                    node.bytes.resize(4, 0);
                    widened = true;
                    continue;
                }
                if !(-32768..=32766).contains(&delta) {
                    out_of_range = true;
                    continue;
                }
                node.bytes = match (branch, node.bytes.len()) {
                    (Branch::Conditional(condition), 2) => vec![0x40 | condition, delta as u8],
                    (Branch::Subroutine, 2) => vec![0x55, delta as u8],
                    (Branch::Conditional(condition), _) => {
                        let [hi, lo] = (delta as i16).to_be_bytes();
                        vec![0x58, condition << 4, hi, lo]
                    }
                    (Branch::Subroutine, _) => {
                        let [hi, lo] = (delta as i16).to_be_bytes();
                        vec![0x5c, 0, hi, lo]
                    }
                };
            }
        }
        if !widened {
            if out_of_range {
                return Err(RelocateError::BranchOutOfRange);
            }
            let bytes = nodes.into_iter().flat_map(|node| node.bytes).collect();
            offsets.pop();
            return Ok((bytes, offsets));
        }
    }
}

pub(crate) fn absolute_target(bytes: &[u8], target: Target, mode: Mode) -> Option<u32> {
    let value = match (bytes[0], bytes[1]) {
        (0x5a | 0x5e, high) => u32::from_be_bytes([0, high, bytes[2], bytes[3]]),
        (0x59 | 0x5d, 8) if target == Target::H8SX => {
            u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]])
        }
        _ => return None,
    };
    // The PC low bit is ignored on instruction fetch (H8SX §1.5.2;
    // corresponding program-counter descriptions in all legacy manuals).
    Some((u64::from(value) % limit(mode)) as u32 & !1)
}

/// Relocates a complete block from `from` to `to`, remapping internal branches.
///
/// Branch targets outside the block (including its end) keep their original
/// addresses. Targets inside the block must name instruction starts. Short
/// branches widen to 16 bits on H8/300H and later when needed; H8/300 refuses
/// an out-of-range branch. Arithmetic respects the mode's PC width, including
/// relative branches across address zero. Blocks themselves must not wrap.
/// Unknown instructions, BRA/S, PC-indexed branches, and absolute transfers
/// into the source block are refused.
/// The input is never modified.
pub fn relocate(
    bytes: &[u8],
    from: u32,
    to: u32,
    target: Target,
    mode: Mode,
) -> Result<Relocated, RelocateError> {
    if !target.supports(mode) {
        return Err(RelocateError::UnsupportedMode);
    }
    address(from, mode)?;
    address(to, mode)?;
    let end = u64::from(from) + bytes.len() as u64;
    if end > limit(mode) {
        return Err(RelocateError::AddressOverflow);
    }
    let mut nodes = Vec::new();
    let mut originals = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let len = insn_len(&bytes[offset..], target, mode)
            .ok_or(RelocateError::UnsupportedInstruction(offset))?;
        let instruction = &bytes[offset..offset + len];
        if let Some(destination) = absolute_target(instruction, target, mode) {
            if destination >= from && u64::from(destination) < end {
                return Err(RelocateError::AbsoluteInternalBranch(destination));
            }
        }
        let branch = classify(instruction, target, offset)?.map(|(kind, delta)| {
            let destination = (i64::from(from) + offset as i64 + len as i64 + i64::from(delta))
                .rem_euclid(limit(mode) as i64) as u32;
            (kind, Destination::Address(destination))
        });
        nodes.push(Node {
            bytes: instruction.to_vec(),
            branch,
        });
        originals.push(offset);
        offset += len;
    }
    for node in &mut nodes {
        node.branch = match node.branch {
            Some((kind, Destination::Address(value)))
                if value >= from && u64::from(value) < end =>
            {
                let index = originals
                    .binary_search(&((value - from) as usize))
                    .map_err(|_| RelocateError::InteriorBranch(value))?;
                Some((kind, Destination::Node(index)))
            }
            other => other,
        };
    }
    let (bytes, offsets) = layout(nodes, to, target, mode)?;
    Ok(Relocated {
        bytes,
        offsets: originals.into_iter().zip(offsets).collect(),
    })
}
