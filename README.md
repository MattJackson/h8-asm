# h8-asm

A **Renesas H8 family decoder, instruction builder and detour installer** for
Rust: H8/300, H8/300H, H8S/2000, H8S/2600 and H8SX.

It is the sibling of [thumb-asm](https://github.com/MattJackson/thumb-asm) and
follows the same contract. It operates on a flat `&[u8]` image (a firmware dump
or a flash region) where file offset and load address are the same number. It
decodes and re-encodes instructions against Renesas' own manuals, and refuses
rather than guesses when a request cannot be done faithfully.

Pure safe Rust: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, no
dependencies beyond `std`.

## Status

**Pre-alpha.** `isa::insn_len` recognizes instruction lengths for H8/300,
H8/300H and H8S, plus four fixed H8SX opcodes. Semantic decoding, encoding and
patching are not implemented. The length tests and their limits are recorded in
[`spec/H8-ISA.md`](spec/H8-ISA.md). The build-out plan is in
[`KICKOFF.md`](KICKOFF.md), and the manuals every encoding will be checked
against are in [`spec/`](spec/README.md).

## Say what you are patching

Every API will take a `Target` (which core) and a `Mode` (which CPU operating
mode). Neither can be recovered from the bytes, and both change what the bytes
mean: absolute-address widths, branch and call targets, vector-table entries,
and which instructions exist at all.

## License

MIT; see [`LICENSE`](LICENSE). The Renesas manuals under `spec/` are Renesas'
copyright, are not covered by that licence, and are excluded from the published
crate. See [`LICENSES/LicenseRef-Renesas-Documentation.txt`](LICENSES/LicenseRef-Renesas-Documentation.txt).
