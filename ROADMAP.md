# Roadmap and remaining assurance work

The source implements shared typed decoding, exact verified encoding and
Renesas rendering for all five H8 targets; flat-image helpers, label assembly,
conservative relocation/analysis and transactional detours are integrated.
The H8SX table includes all 8,493 reviewed source rows. The complete
four-byte sweeps pass for all five targets; mandatory GNU checks and the
reviewed reverse census are documented in docs/CONFORMANCE.md.

Version 0.5.0 is published on crates.io and GitHub. The exact release commit
passed dev and full QA, was promoted through CI to main, and passed full QA
again in release run 36267228252. main is the default branch. The release
archive matches the registry bytes; its Sigstore signature and SLSA provenance
were independently verified. OpenSSF project 14963 confirms the requested
198% tiered score (100% passing, 98% silver). Codecov processes the repository
secret uploads at 100% coverage.

Optional account follow-ups are documented in docs/SETUP.md: Trusted Publishing
can replace the saved release token when account authorization permits it, and
the external REUSE badge depends on the service completing its registration
and scan. Neither is represented as a completed service configuration.

The full EC2 mutation audit and all survivor rechecks are complete. The combined
result is 1,808 mutations: 1,698 caught, 67 surviving, five timeouts and 38
unviable. Every survivor has a documented equivalence assessment; none is
silently excluded. See [the audit report](docs/MUTATION-AUDIT.md) and its complete
machine-readable outcomes for commands, source hashes, reconciliation and
regression fixes. The temporary EC2 instance, security group and key pair were
removed after collecting results.

Longer legacy operand audits are complete. Both initial hosted QA runs passed
all six Linux/macOS/Windows × stable/Rust 1.58 cells, including all five full
four-byte sweeps in each cell. The published commit subsequently passed the
complete six-cell matrix in
both QA run 36266043197 and release run 36267228252; these runs include the
final source/test changes.

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
