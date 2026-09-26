# Contributing

Start with README.md, spec/H8-ISA.md and docs/CONFORMANCE.md. Encoding changes
need a Renesas manual citation, an explicit target/mode rule, boundary tests,
and independent assembler evidence or a narrowly documented oracle divergence.
Do not treat successful decode/encode round trips as independent evidence.

Use the shared Insn/Operand/Reg/Ea vocabulary. Preserve operand widths and
reviewed noncanonical encodings. Encoders must validate by decoding their
output before returning bytes. Unknown encodings and unsafe relocation
requests must return errors; patch failures must leave the image unchanged.
The library must remain safe Rust, dependency-free, documented, and compatible
with Rust 1.58. Follow CODE_OF_CONDUCT.md in project discussions.

New behavior requires tests. Every bug fix must add a regression assertion
that fails before the fix and passes afterward, unless the PR explains why
the failure cannot be reproduced. Use rustfmt formatting and Clippy's checks;
do not suppress a finding merely to pass CI. Keep public documentation and
CHANGELOG.md current with behavioral changes.

Contributions certify the [Developer Certificate of Origin 1.1](https://developercertificate.org/).
Use `git commit -s` to record that certification on new commits. This is a
statement that you have the right to submit the contribution under the project's
license, not a transfer of copyright.

Run the following checks for substantive changes:

- cargo fmt --check
- cargo clippy --all-targets -- -D warnings
- cargo test --all-targets and cargo test --doc
- cargo +1.58.0 test --all-targets
- cargo llvm-cov --all-features --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100
- scripts/build-binutils.sh followed by scripts/conformance.sh
- scripts/exhaustive.sh and the ignored release tests in sx_operand_audit
- RUSTDOCFLAGS='-D warnings' cargo doc --no-deps
- cargo package --allow-dirty

The QA workflow is the authoritative gate, including its OS/MSRV matrix.
Source coverage is a coverage gate, not proof of ISA completeness. Mutation
results and justified survivors belong in ROADMAP.md with reproducible
commands and the tested source identity.

Generated H8SX data must be regenerated through scripts/compile-h8sx-table.py;
--check verifies byte-for-byte reproducibility. Preserve the raw manual facts
and record corrections separately. The extractor requires Poppler, and the
source PDF checksum is pinned. Do not edit generated opcode tables by hand.

Submit changes against dev with the problem, resulting behavior and validation
in the PR description. Promotion to qa runs the complete gate; main is the
release branch. Do not bypass checks or publish a version to repair a CI failure.
Report security issues through SECURITY.md rather than a public issue.

Promotion preserves the exact commit by fast-forwarding dev → qa → main.
The promotion workflow validates the source repository, branch, workflow,
event and current SHA, then dispatches the next stage. dev must pass the fast
gate (including workflow security analysis); qa must pass the complete gate
before main advances. Release re-runs the full gate before publishing.
PR checks do not promote branches, and superseded runs cannot promote old code.
An interrupted promotion can be resumed through promote.yml's dispatch input.
