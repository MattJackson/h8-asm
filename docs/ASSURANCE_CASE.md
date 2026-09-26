# Assurance case

The claim is bounded: accepted instructions follow the documented encoding
rules implemented for the requested Renesas H8 target and mode; encoders
preserve every represented field; checked patch operations validate a complete
plan before modifying the image. This is an evidence-backed engineering claim,
not a proof of hardware behavior or firmware safety.

The library is dependency-free safe Rust with no device, file or network I/O.
It takes caller-owned bytes and typed operands. Encoding rules cite the
bundled Renesas software manuals. H8SX's 8,493 source rows preserve page
provenance, fixed bits, fields and separately reviewed source anomalies.
Generated data has a reproducibility check; field constraints supplement masks.

Evidence includes:

- All two-byte patterns and all 4,294,967,296 four-byte patterns for each of
  five targets, with pinned acceptance/length counts and exact round trips.
- Longer operand witnesses covering register fields and extension boundaries,
  including asymmetric source/destination values and invalid public operands.
- Independent pinned GNU binutils assemble-back, explicit alternative-form
  comparisons, separately counted disassembly-only limitations, and a reviewed
  H8SX reverse-census allow-list. docs/CONFORMANCE.md records counts and limits.
- Tests for mode arithmetic, branch relaxation, internal-target refusal,
  vector widths, overlapping instructions, image boundaries and error atomicity.
- A mandatory 100% source line/region/function gate, MSRV/OS testing, Clippy,
  warning-free rustdoc, package verification and generated-data checks.

Mutation testing deliberately changes expressions and behavior to find gaps
that coverage alone misses. The completed EC2 audit plus regression
rechecks records 1,808 mutations: 1,698 caught, 67 surviving, five timeouts
and 38 unviable. [MUTATION-AUDIT.md](MUTATION-AUDIT.md) documents every survivor
category, added behavioral assertions, tested source hashes and run reconciliation.
Equivalent survivors, detected nontermination and compilation failures are
reported separately; this is not a zero-survivor claim.

The principal limitations are independent of source coverage. A software
manual may contain errors; oracle implementations also have bugs. Long operand
spaces are sampled at specified boundaries rather than fully enumerated.
GNU has no separate middle/maximum-mode selector. Product restrictions may
exceed the generic ISA rules. Execution is not verified on H8 hardware.

Relocation refuses BRA/S, PC-indexed and compound relative forms it cannot
safely rewrite. Analysis reports unresolved control flow and uses supplied
entry points. The caller must establish real instruction boundaries, prevent
other entry paths into overwritten bytes, choose the correct CPU/mode and
validate hook semantics. These preconditions cannot be inferred from a flat
image. See PATCHING.md and ../SECURITY.md.


Build-environment security analysis uses Zizmor 1.30.1 for workflow injection,
credential handling and related GitHub Actions risks, in both dev and QA.
Medium-or-higher findings with medium-or-higher confidence fail the gate.
The promotion workflow's workflow_run trigger is a reviewed exception: it
performs no source checkout or artifact execution and validates repository,
workflow, event, current branch SHA and fast-forward ancestry before API writes.
Checkout credentials are not persisted. Release input values enter scripts
through environment variables instead of executable template interpolation.

Repeatability was checked with two clean release library builds on the same
Rust toolchain and source path using separate CARGO_TARGET_DIR directories.
Both libh8_asm.rlib files had SHA-256
7d87b23bf078ba46de29e23da6a54caa7bf70f3ff75814a4a86b147634ea20af.
This is same-environment build repeatability, not cross-toolchain or
cross-platform reproducibility. The release job separately compares its crate
archive byte-for-byte with the registry artifact before signing it.
