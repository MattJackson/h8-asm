# OpenSSF Best Practices evidence

Registration and a project identifier are pending. No passing badge is claimed.
This is the evidence checklist for completing the questionnaire after the
first release and account setup; it must reflect actual results at submission.

| Area | Repository evidence | Current limit |
|---|---|---|
| Purpose and interface | README.md, public rustdoc, docs/PATCHING.md | Pre-release; not hardware execution validation |
| Licensing | LICENSES/, REUSE.toml, REUSE lint | Renesas manuals retain their separate license |
| Contribution process | CONTRIBUTING.md, CODE_OF_CONDUCT.md, GOVERNANCE.md | One maintainer |
| Public development | GitHub repository, dev → qa PRs | main/release promotion pending |
| Change history | CHANGELOG.md and git | 0.5.0 release candidate; not published yet |
| Security reports | SECURITY.md, enabled private vulnerability reporting | No guaranteed response-time commitment |
| Automated tests | Dev/QA workflows, required QA gate | Final mutation audit still in progress |
| Independent conformance | docs/CONFORMANCE.md, pinned GNU oracle | Recorded GNU defects and mode limits |
| Exhaustive encoding checks | spec/H8-ISA.md, scripts/exhaustive.sh | Long extension Cartesian products not exhausted |
| Static analysis | Clippy with denied warnings, safe-Rust policy | No formal proof |
| Dependency posture | Zero runtime dependencies; pinned CI actions | Build/test tools remain external dependencies |
| Coverage | Mandatory 100% line/region/function gate | Coverage is not ISA or behavioral completeness |
| Release provenance | release.yml verification and attestation steps | First publication/authentication pending |
| Supply-chain review | Scheduled/manual OpenSSF Scorecard workflow | A score does not certify firmware safety |

Use the actual repository URL and real maintainer account when registering.
Do not copy another project's ID, mutation counts, badge state or release
claims. Record the assigned project ID and questionnaire completion date here
only after the service confirms them. Required evidence URLs should reference
the released main branch once that branch exists.
