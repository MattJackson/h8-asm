# Roadmap and remaining assurance work

The source implements shared typed decoding, exact verified encoding and
Renesas rendering for all five H8 targets; flat-image helpers, label assembly,
conservative relocation/analysis and transactional detours are integrated.
The H8SX table includes all 8,493 reviewed source rows. The complete
four-byte sweeps pass for all five targets; mandatory GNU checks and the
reviewed reverse census are documented in docs/CONFORMANCE.md.

Before the first release:

- Finish the active EC2 mutation audit, assess every survivor, add missing
  behavioral tests, and record the tested revision and reproduction command.
- Finish the longer legacy operand audit and final OS/MSRV/package/source
  coverage checks after all changes.
- Configure GitHub branches and release protection, trusted publishing,
  Codecov and OpenSSF; record any account steps still requiring interactive
  authentication in docs/SETUP.md. No first release has been published yet.

The current mutation run uses cargo-mutants 27.1.0 on an isolated c7i.8xlarge
Ubuntu 24.04 worker, release profile, 16 jobs and a 32-task jobserver. It
includes 1,801 generated mutations. Source snapshot SHA-256:
e2a21690d16097fe824f14ad078251194f1471da591f1c0c108693cc4caa3c6c.
Command: cargo mutants --profile release -j 16 --jobserver-tasks 32
--timeout 180 --build-timeout 180. It is still running; no full result is
claimed. Later regression tests must be audited separately from that snapshot.

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
