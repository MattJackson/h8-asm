# Roadmap and remaining assurance work

The source implements shared typed decoding, exact verified encoding and
Renesas rendering for all five H8 targets; flat-image helpers, label assembly,
conservative relocation/analysis and transactional detours are integrated.
The H8SX table includes all 8,493 reviewed source rows. The complete
four-byte sweeps pass for all five targets; mandatory GNU checks and the
reviewed reverse census are documented in docs/CONFORMANCE.md.

Before the first release (now 0.5.0, as requested by the maintainer):

- Run the final release candidate through hosted QA, including the new vector
  alignment regression and mutation assertions. The Codecov OIDC upload has
  succeeded and its first report is processed.
- Promote dev → qa → main with required checks and release protection; publish
  the verified crate and signed/attested release artifacts.
- Finish Trusted Publishing and OpenSSF account setup; record actual service
  state in docs/SETUP.md. A publishing token is saved locally and in the release
  environment, so unattended token publication is configured as a fallback.

The full EC2 mutation audit and all survivor rechecks are complete. The combined
result is 1,808 mutations: 1,698 caught, 67 surviving, five timeouts and 38
unviable. Every survivor has a documented equivalence assessment; none is
silently excluded. See [the audit report](docs/MUTATION-AUDIT.md) and its complete
machine-readable outcomes for commands, source hashes, reconciliation and
regression fixes. The temporary EC2 instance, security group and key pair were
removed after collecting results.

Longer legacy operand audits are complete. Both initial hosted QA runs passed
all six Linux/macOS/Windows × stable/Rust 1.58 cells, including all five full
four-byte sweeps in each cell. The final candidate must pass again after the
last source/test changes; those earlier runs do not certify newer changes.

Deliberate current limitations:

- BRA/S, PC-indexed, bit-test relative and MOVSD.B relocation is refused;
  dynamic register state and delay-slot safety are not guessed.
- Reachability is conservative; function lookup needs caller-supplied entries.
- Generic ISA support does not establish permission for product-restricted
  operations or validate execution on hardware.
- Long extension fields are tested at stated boundaries, not over their full
  Cartesian product. Independent GNU coverage is bounded by its supported
  modes and documented defects.

Future work may add hardware/emulator execution checks, more safe relocation
forms with explicit preconditions, and product-level instruction restrictions.
Those additions must preserve exact encodings and existing safety refusals.

## Planning horizon: September 2026–September 2027

Over the next year, the intended maintenance priorities are regression fixes,
keeping the independent oracle/tool versions reviewed, preserving Rust 1.58
compatibility, and keeping the automated release path and public documentation
current. New instruction or relocation support needs manual evidence and
independent checks before it can replace a refusal.

Hardware/emulator comparisons and explicit product restrictions are possible
follow-up investigations, not promised deliveries. There is no plan to add
device access, execute firmware, infer a CPU/mode from bytes, add runtime
dependencies, or guess the safety of dynamic/delayed control flow. Schedule
and scope depend on maintainer capacity and concrete consumer requirements.
