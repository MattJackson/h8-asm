# OpenSSF Best Practices evidence

The real project is [14963](https://www.bestpractices.dev/projects/14963).
The target is a 198% tiered score: passing complete, silver 98%. On 2026-09-26
the maintainer confirmed both personal security-knowledge declarations and
both forms were submitted. The service awarded the passing badge: 100% passing,
96% silver, and 196% tiered. Verified release signatures remain pending before
the target score can be claimed.

[The prepared answers](openssf-best-practices.json) contain 115 distinct
passing/silver criteria and their evidence, not a copy of the reference
project's attestations. They are a dated working assessment; the service's
actual saved result is authoritative. A single maintainer does not establish
one-week access continuity, so that criterion is explicitly Unmet. The bus
factor and unsigned git tags are also stated honestly. Workflow security
analysis uses Zizmor; release signatures remain pending until verified.

| Area | Repository evidence | Current limit |
|---|---|---|
| Purpose and interface | README.md, public rustdoc, docs/PATCHING.md | Pre-release; not hardware execution validation |
| Licensing | LICENSES/, REUSE.toml, REUSE lint | Renesas manuals retain their separate license |
| Contribution process | CONTRIBUTING.md, CODE_OF_CONDUCT.md, GOVERNANCE.md | One maintainer |
| Public development | GitHub repository, automatic dev → qa promotion | Automatic promotion configured; full chain verification pending |
| Change history | CHANGELOG.md and git | 0.5.0 release candidate; not published yet |
| Security reports | SECURITY.md, enabled private vulnerability reporting | No guaranteed response-time commitment |
| Automated tests | Dev/QA workflows, required QA gate | Completed mutation audit: docs/MUTATION-AUDIT.md |
| Independent conformance | docs/CONFORMANCE.md, pinned GNU oracle | Recorded GNU defects and mode limits |
| Exhaustive encoding checks | spec/H8-ISA.md, scripts/exhaustive.sh | Long extension Cartesian products not exhausted |
| Static analysis | Clippy with denied warnings, safe-Rust policy | No formal proof |
| Dependency posture | Zero runtime dependencies; pinned CI actions | Build/test tools remain external dependencies |
| Coverage | Mandatory 100% line/region/function gate | Coverage is not ISA or behavioral completeness |
| Release provenance | release.yml verification and attestation steps | Token authentication configured; publication pending |
| Supply-chain review | Scheduled/manual OpenSSF Scorecard workflow | A score does not certify firmware safety |

Use the actual repository URL and real maintainer account when registering.
Do not copy another project's ID, mutation counts, badge state or release
claims. Record questionnaire completion here only after the service confirms it. Required evidence URLs should reference
the released main branch once that branch exists.
