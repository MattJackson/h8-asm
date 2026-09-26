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
that coverage alone misses. The first complete EC2 audit is in progress;
its outcome must be recorded honestly before release. A surviving equivalent
mutation and a missing behavioral assertion are different findings. Do not
claim that a pending or partial run passed.

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
