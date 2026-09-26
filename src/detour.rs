//! Transactional hooks with trampolines in flat images.
//!
//! JMP encodings follow H8/300 §2 and H8SX REJ09B0102 §2.4: `5a aa:24`
//! (H8/300 reserves the high address byte) and H8SX `5908 aa:32`.
//! A trampoline preserves recognized displaced instructions and jumps back.

use crate::{
    isa::insn_len,
    relocate::{self, RelocateError},
    Mode, Target,
};

/// A detour refusal. Installation never changes the image on error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetourError {
    /// A site, hook or trampoline range does not lie in the image.
    OutOfBounds,
    /// The hook site, trampoline or hook entry overlap.
    Overlap,
    /// The entire trampoline area is not erased (`0xff`).
    NotFree,
    /// The hook site is immediately after a possible H8SX delayed branch.
    DelaySlot,
    /// Instruction relocation or CPU/address validation failed.
    Relocation(RelocateError),
}

/// A validated but unapplied detour plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetourPlan {
    /// Hook-site offset.
    pub site: u32,
    /// Trampoline offset; call this to execute the displaced instructions.
    pub trampoline: u32,
    /// First original instruction after the overwritten block.
    pub resume: u32,
    /// Bytes installed at the hook site, including NOP padding.
    pub site_bytes: Vec<u8>,
    /// Relocated block followed by an absolute jump back.
    pub trampoline_bytes: Vec<u8>,
}

// Both callers have already validated the destination and CPU context.
fn jump(destination: u32, target: Target, mode: Mode) -> Vec<u8> {
    let [a, b, c, d] = destination.to_be_bytes();
    let bytes = if mode == Mode::Maximum {
        vec![0x59, 0x08, a, b, c, d]
    } else {
        vec![0x5a, b, c, d]
    };
    // Independently dispatch the chosen form through length recognition.
    assert_eq!(insn_len(&bytes, target, mode), Some(bytes.len()));
    bytes
}

/// Plans a hook to `hook`, storing displaced instructions at `trampoline`.
///
/// The caller must supply a known instruction boundary and ensure no other
/// entry points enter the overwritten interior. The site is extended to whole
/// instructions, and the trampoline must be erased (`0xff`). Mode and all
/// address/range/overlap checks happen before any mutation. A possible delay
/// slot immediately before the site is conservatively refused.
pub fn plan(
    image: &[u8],
    site: u32,
    hook: u32,
    trampoline: u32,
    target: Target,
    mode: Mode,
) -> Result<DetourPlan, DetourError> {
    if !target.supports(mode) {
        return Err(DetourError::Relocation(RelocateError::UnsupportedMode));
    }
    for value in [site, hook, trampoline] {
        relocate::address(value, mode).map_err(DetourError::Relocation)?;
        if u64::from(value) >= image.len() as u64 {
            return Err(DetourError::OutOfBounds);
        }
    }
    let site_offset = site as usize;
    if target == Target::H8SX
        && site_offset >= 2
        && image[site_offset - 2] == 0x40
        && image[site_offset - 1] & 1 != 0
    {
        return Err(DetourError::DelaySlot);
    }
    let mut site_bytes = jump(hook, target, mode);
    let mut len = 0;
    while len < site_bytes.len() {
        // Each previous length was checked against the remaining image.
        let remaining = &image[site_offset + len..];
        len += insn_len(remaining, target, mode).ok_or(DetourError::Relocation(
            RelocateError::UnsupportedInstruction(len),
        ))?;
    }
    let resume = u64::from(site) + len as u64;
    if resume >= relocate::limit(mode) {
        return Err(DetourError::Relocation(RelocateError::AddressOverflow));
    }
    let displaced = &image[site_offset..site_offset + len];
    let relocated = relocate::relocate(displaced, site, trampoline, target, mode)
        .map_err(DetourError::Relocation)?;
    let mut trampoline_bytes = relocated.bytes;
    trampoline_bytes.extend(jump(resume as u32, target, mode));
    let trampoline_end = u64::from(trampoline) + trampoline_bytes.len() as u64;
    if trampoline_end > image.len() as u64 || trampoline_end > relocate::limit(mode) {
        return Err(DetourError::OutOfBounds);
    }
    let site_range = u64::from(site)..resume;
    let trampoline_range = u64::from(trampoline)..trampoline_end;
    if site_range.start < trampoline_range.end && trampoline_range.start < site_range.end
        || site_range.contains(&u64::from(hook))
        || trampoline_range.contains(&u64::from(hook))
    {
        return Err(DetourError::Overlap);
    }
    if image[trampoline as usize..trampoline_end as usize]
        .iter()
        .any(|&byte| byte != 0xff)
    {
        return Err(DetourError::NotFree);
    }
    // H8 NOP is the all-zero word; len and jump length are always even.
    site_bytes.resize(len, 0);
    Ok(DetourPlan {
        site,
        trampoline,
        resume: resume as u32,
        site_bytes,
        trampoline_bytes,
    })
}

/// Plans and installs a hook atomically, returning the installed plan.
/// No byte changes until every check in [`plan`] has succeeded.
pub fn tramp(
    image: &mut [u8],
    site: u32,
    hook: u32,
    trampoline: u32,
    target: Target,
    mode: Mode,
) -> Result<DetourPlan, DetourError> {
    let plan = plan(image, site, hook, trampoline, target, mode)?;
    let site = plan.site as usize;
    let trampoline = plan.trampoline as usize;
    image[site..site + plan.site_bytes.len()].copy_from_slice(&plan.site_bytes);
    image[trampoline..trampoline + plan.trampoline_bytes.len()]
        .copy_from_slice(&plan.trampoline_bytes);
    Ok(plan)
}
