# h8-asm

[![Development checks](https://github.com/MattJackson/h8-asm/actions/workflows/dev.yml/badge.svg?branch=dev)](https://github.com/MattJackson/h8-asm/actions/workflows/dev.yml)
[![QA](https://github.com/MattJackson/h8-asm/actions/workflows/qa.yml/badge.svg?branch=qa)](https://github.com/MattJackson/h8-asm/actions/workflows/qa.yml)
[![Codecov](https://codecov.io/gh/MattJackson/h8-asm/graph/badge.svg?branch=dev)](https://app.codecov.io/gh/MattJackson/h8-asm)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/MattJackson/h8-asm/badge)](https://securityscorecards.dev/viewer/?uri=github.com/MattJackson/h8-asm)
[![REUSE](https://api.reuse.software/badge/github.com/MattJackson/h8-asm)](https://api.reuse.software/info/github.com/MattJackson/h8-asm)

A **Renesas H8 family decoder, instruction builder and detour
installer** for Rust: H8/300, H8/300H, H8S/2000, H8S/2600 and H8SX.

It is the sibling of [thumb-asm](https://github.com/MattJackson/thumb-asm) and
follows the same contract. It operates on a flat `&[u8]` image (a firmware dump
or a flash region) where file offset and load address are the same number. It
uses Renesas' own manuals as the encoding authority and refuses unsupported
requests rather than guessing.

Pure safe Rust: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, no
dependencies beyond `std`.

## Status

**0.5.0 release candidate.** `isa::insn_len` uses the legacy opcode maps and a generated
H8SX grammar covering all 8,493 printed §2.4 rows, with explicit field
restrictions and reviewed source anomalies. The shared typed vocabulary and
`decode_insn`/`encode_insn`/`disassemble_insn` cover all five targets, including
memory operands, register groups, control registers and H8SX MOVA's nested
indexed operands. The original H8/300 and H8SX subset APIs are retained.
`Asm`, relocation, transactional detours, big-endian image helpers, command
tables, and conservative control-flow analysis now operate on recognized
instruction families. Delayed/PC-indexed relocation and other named unsafe
cases are refused; see [`docs/PATCHING.md`](docs/PATCHING.md). The coverage map and limits are in
[`spec/H8-ISA.md`](spec/H8-ISA.md). Remaining release work is in
[`ROADMAP.md`](ROADMAP.md), and the encoding reference manuals are in [`spec/`](spec/README.md).

The shared legacy codec independently assembles all 215,149 supported
two-byte words back to exact bytes across its four cores. Extended operand
probes also pass, with narrowly identified alias choices and GNU divergences
reported separately. The shared H8SX codec assembles all 56,080 supported
two-byte words exactly. Five constrained witnesses per manual row yield
42,315 additional exact results, 12 verified alternative forms and 18
GNU limitations checked independently through disassembly.
First-word length probes compare all 65,536 words per target with a zero suffix.
Those probes are narrower than full ISA conformance. Reproduce them with
`scripts/build-binutils.sh` followed by `scripts/conformance.sh`; QA requires
the same checks. See [`docs/CONFORMANCE.md`](docs/CONFORMANCE.md).

The encoders decode their output and assert the same instruction and length
before returning bytes. This detects internal disagreement, but is not an
independent proof of an encoding. The normal suite passes on Rust 1.58 and
has a 100% line/region/function coverage gate. Source coverage measures the
implemented code, not completion of the instruction set.

## Say what you are patching

Instruction-aware APIs take a `Target` (which core) and a `Mode` (which CPU
operating mode). Neither can be recovered from the bytes, and both change what the bytes
mean: absolute-address widths, branch and call targets, vector-table entries,
and which instructions exist at all.

## License

MIT; see [`LICENSE`](LICENSE). The Renesas manuals under `spec/` are Renesas'
copyright, are not covered by that licence, and are excluded from the published
crate. See [`LICENSES/LicenseRef-Renesas-Documentation.txt`](LICENSES/LicenseRef-Renesas-Documentation.txt).
