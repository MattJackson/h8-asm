# Changelog

All notable changes to this crate are recorded here. Versions follow semantic
versioning; before 1.0, incompatible public changes increase the minor version.

## [Unreleased]

### Added

- Shared typed decoding, verified encoding and Renesas rendering for H8/300,
  H8/300H, H8S/2000, H8S/2600 and H8SX, retaining the original subset APIs.
- Reproducible H8SX extraction and generated grammar/operands for all 8,493
  reviewed manual rows, including nested MOVA operands and explicit aliases.
- Full two-byte and four-byte sweeps across all five targets, pinned censuses,
  wider operand-field checks and adversarial typed-input tests.
- Checksum-pinned GNU binutils oracle with exact assemble-back, explicit
  divergences, normal-mode checks and a reviewed H8SX reverse census.
- Big-endian search, checked reads, transactional image edits and command tables.
- Label assembly and iterative branch relaxation; relocation remaps internal
  targets and refuses unsafe dynamic, delayed or compound control flow.
- Transactional detour planning/installation and conservative cross-reference,
  vector, reachability and known-entry analysis.
- Required coverage, MSRV/OS, conformance, exhaustive-sweep and package gates;
  security, contribution, governance and assurance documentation.

### Corrected during validation

- Reserved H8S register-group starts and odd relative branch fields are refused.
- Manual-valid zero five-bit H8SX shift counts are accepted.
- Reviewed H8SX source anomalies are preserved separately from corrections.
- MOVSD.B's relative control flow is refused by unsupported relocation/analysis.
- Absolute/vector destinations ignore PC bit zero during control-flow analysis;
  relocation detects the resulting effective target inside an overwritten block.
- H8SX MOVA retains RnH versus RnL and En versus Rn register identity despite
  GNU assembler shortening defects.
