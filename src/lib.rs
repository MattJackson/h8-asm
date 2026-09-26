//! A decoder, disassembler, assembler and detour installer for the Renesas H8
//! family (H8/300, H8/300H, H8S/2000, H8S/2600 and H8SX), operating on a flat
//! big-endian `&[u8]` image where file offset and load address are the same
//! number.
//!
//! **Pre-alpha.** [`isa::insn_len`] recognizes lengths through H8S and selected
//! H8SX opcode families. [`isa::decode::decode`] decodes a small H8/300 subset;
//! [`isa::sx_semantic::decode`] covers selected H8SX control flow, register
//! operations, immediates, and absolute byte moves. Both subsets have matching
//! disassemblers and verified encoders. [`Asm`], [`relocate`], [`detour`],
//! [`image`], and [`analysis`] operate on recognized instruction families. Every API is parameterised by
//! which core the bytes were
//! written for ([`Target`]), and which CPU operating mode it runs in
//! ([`Mode`]). Both change what a given byte sequence means, so neither is
//! optional context that can be defaulted away later. See `KICKOFF.md` for
//! the build-out plan.
//!
//! Every encoding claim in this crate cites its section in Renesas' software
//! manuals under `spec/` (H8/300 ADE-602-025, H8/300H REJ09B0213, H8S
//! REJ09B0139, H8SX REJ09B0102).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod analysis;
pub mod asm;
pub mod detour;
pub mod image;
pub mod isa;
pub use image::{
    find, find_free_space, insert, read_u16, read_u32, read_u8, write, CommandRecord, CommandTable,
    ImageError, Needle, TableError,
};
pub mod relocate;
pub use asm::Asm;

/// The H8 core an image was written for.
///
/// Each generation is a superset of the one before it, so the same bytes can
/// decode differently, or only decode at all, depending on the target. This
/// is the same "confident wrong answer" risk that `thumb_asm::isa::Target`
/// exists for.
///
/// `#[non_exhaustive]` so that adding a core, or splitting one (the H8SX
/// manual notes product-specific differences), is not a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Target {
    /// H8/300: 16-bit registers, 64 KB address space, no operating modes
    /// (ADE-602-025).
    H8_300,
    /// H8/300H: adds the 32-bit `ERn` registers and normal/advanced modes
    /// (REJ09B0213).
    H8_300H,
    /// H8S/2000: adds `EXR`, `LDM`/`STM` and `TAS` to the H8/300H set
    /// (REJ09B0139).
    H8S2000,
    /// H8S/2600: H8S/2000 plus the multiply-accumulate (`MAC`) family
    /// (REJ09B0139).
    H8S2600,
    /// H8SX: the superset, with four operating modes and the extended
    /// addressing modes (REJ09B0102).
    H8SX,
}

/// The CPU operating mode, which sets the width of the address space, of
/// branch and call targets, of vector-table entries and of stacked `PC`s.
///
/// The mode is fixed by the chip's mode pins or its configuration and cannot be
/// read from the image, so the caller has to state it. Guessing wrong corrupts
/// every absolute address and every vector-table decode, not just a single
/// instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Mode {
    /// 64 KB address space, 16-bit branch addresses. This is the only mode
    /// H8/300 has.
    Normal,
    /// H8SX only: 16 MB program area, 64 KB data area (H8SX §1.2.2).
    Middle,
    /// A 16 MB program area on every core that has this mode. The data area is
    /// 16 MB on H8/300H (REJ09B0213 §1.1) and 4 GB on H8S (REJ09B0139 §1.2) and
    /// H8SX (REJ09B0102 §1.2.3).
    Advanced,
    /// H8SX only: 4 GB program and data areas (H8SX §1.2.4).
    Maximum,
}

impl Target {
    /// Whether `mode` exists on this core.
    ///
    /// H8/300 has no modes and is treated as permanently [`Mode::Normal`].
    /// H8/300H and H8S have normal and advanced modes (H8S REJ09B0139 §1.2).
    /// H8SX has all four (REJ09B0102 §1.2).
    ///
    /// ```
    /// use h8_asm::{Mode, Target};
    /// assert!(Target::H8SX.supports(Mode::Maximum));
    /// assert!(!Target::H8S2600.supports(Mode::Middle));
    /// ```
    pub fn supports(self, mode: Mode) -> bool {
        match self {
            Target::H8_300 => mode == Mode::Normal,
            Target::H8_300H | Target::H8S2000 | Target::H8S2600 => {
                matches!(mode, Mode::Normal | Mode::Advanced)
            }
            Target::H8SX => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGETS: [Target; 5] = [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
        Target::H8SX,
    ];
    const MODES: [Mode; 4] = [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum];

    /// The whole target × mode table, written out as a literal, so a change to
    /// `supports` fails here rather than slipping through.
    #[test]
    fn mode_support_table() {
        let expected = [
            [true, false, false, false],
            [true, false, true, false],
            [true, false, true, false],
            [true, false, true, false],
            [true, true, true, true],
        ];
        for (t, row) in TARGETS.iter().zip(expected.iter()) {
            for (m, &want) in MODES.iter().zip(row.iter()) {
                assert_eq!(t.supports(*m), want, "{t:?} / {m:?}");
            }
        }
    }
}
