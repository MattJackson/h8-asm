//! Conservative control-flow analysis of recognized H8 instructions.
//!
//! Walks start at caller-supplied instruction boundaries. References use the
//! mode's PC width (H8SX §1.2, §1.8.8–11); vector entries are big-endian and
//! two bytes in normal mode, four otherwise. Indirect register destinations
//! are reported as unresolved. Delay-slot control flow is refused.

use crate::{
    image::{read_u16, read_u32},
    isa::insn_len,
    relocate::{self, Branch},
    Mode, Target,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::Range;

/// Whether a reference is a branch or a returning call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrefKind {
    /// A branch/jump, possibly conditional.
    Branch,
    /// A subroutine call or software trap, with a fallthrough return address.
    Call,
}

/// A code reference, including unresolved register/vector destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xref {
    /// Address of the instruction containing the reference.
    pub from: u32,
    /// Resolved destination, or `None` when runtime state/data is unavailable.
    pub to: Option<u32>,
    /// Branch or call.
    pub kind: XrefKind,
    /// Bcc condition nibble, if applicable (0 always, 1 never).
    pub condition: Option<u8>,
    /// Address of the vector entry for a memory-indirect transfer.
    pub vector: Option<u32>,
}

/// A refusal to infer control flow from incomplete or inconsistent evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError {
    /// Unsupported CPU/mode or invalid code address.
    Context(relocate::RelocateError),
    /// The requested range or an instruction lies outside the image.
    OutOfBounds,
    /// Unknown or truncated instruction at this address.
    UnsupportedInstruction(u32),
    /// A delayed branch requires analysis not implemented here.
    DelaySlot(u32),
    /// Reachability cannot resolve this jump's destination.
    UnresolvedBranch(u32),
    /// Two discovered instructions overlap at this address.
    OverlappingInstructions(u32),
    /// More than one supplied entry reaches the requested instruction.
    AmbiguousEntry(u32),
}

enum Flow {
    Next,
    Return,
    Transfer(Xref),
}

fn context(target: Target, mode: Mode) -> Result<(), AnalysisError> {
    if !target.supports(mode) {
        Err(AnalysisError::Context(
            relocate::RelocateError::UnsupportedMode,
        ))
    } else {
        Ok(())
    }
}

fn flow(bytes: &[u8], at: u32, target: Target, mode: Mode) -> Result<Flow, AnalysisError> {
    let hi = bytes[0];
    let lo = bytes[1];
    if target == Target::H8SX && hi == 0x40 && lo & 1 != 0 {
        return Err(AnalysisError::DelaySlot(at));
    }
    if target == Target::H8SX && crate::isa::sx_complex_relative(bytes) {
        return Err(AnalysisError::UnresolvedBranch(at));
    }
    let next = (u64::from(at) + bytes.len() as u64) % relocate::limit(mode);
    if matches!(hi, 0x40..=0x4f | 0x55 | 0x58 | 0x5c) {
        let (kind, delta) = relocate::classify(bytes, target, 0)
            .expect("validated direct branch; delay slots already refused")
            .unwrap();
        let to = (next as i64 + i64::from(delta)).rem_euclid(relocate::limit(mode) as i64) as u32;
        let (kind, condition) = match kind {
            Branch::Conditional(code) => (XrefKind::Branch, Some(code)),
            Branch::Subroutine => (XrefKind::Call, None),
        };
        return Ok(Flow::Transfer(Xref {
            from: at,
            to: Some(to),
            kind,
            condition,
            vector: None,
        }));
    }
    if matches!(hi, 0x54 | 0x56) {
        return Ok(Flow::Return);
    }
    if matches!(hi,0x59..=0x5b|0x5d..=0x5f|0x57) {
        let kind = if matches!(hi, 0x5d..=0x5f | 0x57) {
            XrefKind::Call
        } else {
            XrefKind::Branch
        };
        let vector = if matches!(hi, 0x5b | 0x5f) {
            Some(u32::from(lo))
        } else if target == Target::H8SX && matches!(hi, 0x59 | 0x5d) && lo & 0x80 != 0 {
            Some(u32::from(lo) * if mode == Mode::Normal { 2 } else { 4 })
        } else {
            None
        };
        return Ok(Flow::Transfer(Xref {
            from: at,
            to: relocate::absolute_target(bytes, target, mode),
            kind,
            condition: None,
            vector,
        }));
    }
    Ok(Flow::Next)
}

fn resolve(flow: Flow, image: &[u8], mode: Mode) -> Flow {
    match flow {
        Flow::Transfer(mut reference) => {
            if let Some(vector) = reference.vector {
                reference.to = if mode == Mode::Normal {
                    read_u16(image, vector as usize).map(|value| u32::from(value) & !1)
                } else {
                    read_u32(image, vector as usize)
                        .map(|value| (u64::from(value) % relocate::limit(mode)) as u32 & !1)
                };
            }
            Flow::Transfer(reference)
        }
        other => other,
    }
}

/// Lists control-flow references in a complete, sequential code range.
/// The range must begin at a known boundary and end after a whole instruction.
/// Missing vector data leaves `to` unresolved. Register-indirect and indexed
/// references are also unresolved; no runtime register values are guessed.
pub fn xrefs(
    image: &[u8],
    range: Range<usize>,
    target: Target,
    mode: Mode,
) -> Result<Vec<Xref>, AnalysisError> {
    context(target, mode)?;
    let bytes = image.get(range.clone()).ok_or(AnalysisError::OutOfBounds)?;
    if range.start as u64 >= relocate::limit(mode)
        || range.start & 1 != 0
        || range.end as u64 > relocate::limit(mode)
    {
        return Err(AnalysisError::Context(
            relocate::RelocateError::InvalidAddress,
        ));
    }
    let mut offset = 0;
    let mut references = Vec::new();
    while offset < bytes.len() {
        let at = (range.start + offset) as u32;
        let len = insn_len(&bytes[offset..], target, mode)
            .ok_or(AnalysisError::UnsupportedInstruction(at))?;
        if let Flow::Transfer(reference) = resolve(
            flow(&bytes[offset..offset + len], at, target, mode)?,
            image,
            mode,
        ) {
            references.push(reference);
        }
        offset += len;
    }
    Ok(references)
}

/// Finds the instruction starts reachable within a function, sorted by address.
/// Calls contribute their return address, not their callee's body. Bcc explores
/// both outcomes except BRA/BRN. Returns stop the walk. An unresolved jump,
/// unknown instruction or overlapping decode fails the entire walk rather than
/// returning a falsely complete function. Finite images and a visited map bound
/// the work even in loops. BRA/S is refused.
pub fn reachable(
    image: &[u8],
    entry: u32,
    target: Target,
    mode: Mode,
) -> Result<Vec<u32>, AnalysisError> {
    context(target, mode)?;
    let mut pending = VecDeque::from(vec![entry]);
    let mut visited = BTreeMap::<u32, usize>::new();
    while let Some(at) = pending.pop_front() {
        relocate::address(at, mode).map_err(AnalysisError::Context)?;
        if visited.contains_key(&at) {
            continue;
        }
        if let Some((&previous, &len)) = visited.range(..at).next_back() {
            if u64::from(previous) + len as u64 > u64::from(at) {
                return Err(AnalysisError::OverlappingInstructions(at));
            }
        }
        let remaining = image.get(at as usize..).ok_or(AnalysisError::OutOfBounds)?;
        let len =
            insn_len(remaining, target, mode).ok_or(AnalysisError::UnsupportedInstruction(at))?;
        let end = u64::from(at) + len as u64;
        if end > relocate::limit(mode) {
            return Err(AnalysisError::Context(
                relocate::RelocateError::AddressOverflow,
            ));
        }
        if let Some((&following, _)) = visited.range(at..).next() {
            if u64::from(following) < end {
                return Err(AnalysisError::OverlappingInstructions(following));
            }
        }
        let next = (end % relocate::limit(mode)) as u32;
        match resolve(flow(&remaining[..len], at, target, mode)?, image, mode) {
            Flow::Next => pending.push_back(next),
            Flow::Return => {}
            Flow::Transfer(reference) => {
                if reference.kind == XrefKind::Call {
                    pending.push_back(next);
                } else {
                    if reference.condition != Some(1) {
                        pending.push_back(reference.to.ok_or(AnalysisError::UnresolvedBranch(at))?);
                    }
                    if reference.condition.is_some() && reference.condition != Some(0) {
                        pending.push_back(next);
                    }
                }
            }
        }
        visited.insert(at, len);
    }
    Ok(visited.into_keys().collect())
}

/// Finds the unique supplied entry whose local control-flow graph reaches `at`.
/// This does not guess prologues or scan backward through variable-length
/// instructions. Entries must be known boundaries, and their graphs must be
/// analyzable by [`reachable`]. Duplicate entries are ignored; ambiguity is an
/// error. `at` must name an instruction start, not a byte within an instruction.
pub fn function_start(
    image: &[u8],
    at: u32,
    entries: &[u32],
    target: Target,
    mode: Mode,
) -> Result<Option<u32>, AnalysisError> {
    context(target, mode)?;
    relocate::address(at, mode).map_err(AnalysisError::Context)?;
    let mut found = None;
    for &entry in entries.iter().collect::<BTreeSet<_>>() {
        if reachable(image, entry, target, mode)?
            .binary_search(&at)
            .is_ok()
        {
            if found.is_some() {
                return Err(AnalysisError::AmbiguousEntry(at));
            }
            found = Some(entry);
        }
    }
    Ok(found)
}
