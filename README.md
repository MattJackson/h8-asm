# h8-asm

An in-progress **Renesas H8 family decoder, instruction builder and detour
installer** for Rust: H8/300, H8/300H, H8S/2000, H8S/2600 and H8SX.

It is the sibling of [thumb-asm](https://github.com/MattJackson/thumb-asm) and
follows the same contract. It operates on a flat `&[u8]` image (a firmware dump
or a flash region) where file offset and load address are the same number. It
uses Renesas' own manuals as the encoding authority and refuses unsupported
requests rather than guessing.

Pure safe Rust: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, no
dependencies beyond `std`.

## Status

**Pre-alpha.** `isa::insn_len` recognizes instruction lengths through H8S and
selected H8SX families. A typed H8/300 decoder, disassembler and encoder
cover 2,692 two-byte encodings; a separate H8SX decoder, disassembler, and
encoder cover selected control flow, byte immediates, and absolute byte moves.
Relocation and detour installation are
not implemented. The coverage map and limits are in
[`spec/H8-ISA.md`](spec/H8-ISA.md). The build-out plan is in
[`KICKOFF.md`](KICKOFF.md), and the manuals every encoding will be checked
against are in [`spec/`](spec/README.md).

The H8/300 semantic subset has an independent assemble-back check: its 2,692
decoded words render as Renesas assembly, are assembled and linked with GNU
binutils 2.47, and reproduce the original bytes exactly. Opt-in tests also
compare all 65,536 first words per target against binutils with a zero-filled
suffix. A second assemble-back test matches all 10,369 semantically decoded
H8SX two-byte words and 140 selected four-byte forms against binutils 2.47.
Those probes are narrower than full ISA conformance.

## Say what you are patching

Every API will take a `Target` (which core) and a `Mode` (which CPU operating
mode). Neither can be recovered from the bytes, and both change what the bytes
mean: absolute-address widths, branch and call targets, vector-table entries,
and which instructions exist at all.

## License

MIT; see [`LICENSE`](LICENSE). The Renesas manuals under `spec/` are Renesas'
copyright, are not covered by that licence, and are excluded from the published
crate. See [`LICENSES/LicenseRef-Renesas-Documentation.txt`](LICENSES/LicenseRef-Renesas-Documentation.txt).
