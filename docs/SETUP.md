# Release and service setup

Use the maintainer identity Matthew Jackson, matthew@pq.io. This page records
real configuration and the remaining account steps. The implementation's
coverage and conformance gates do not depend on a badge claiming completion.

## GitHub

The public repository is https://github.com/MattJackson/h8-asm. Sources are
on dev; qa runs the full verification workflow. dev is the temporary default
branch until the first release. Promotion follows dev → qa → main.

Completed configuration:

- Private vulnerability reporting is enabled; SECURITY.md gives both the
  private advisory route and the maintainer email.
- dev rejects deletion and force pushes and requires linear history, including
  for administrators.
- qa requires a pull request and the successful qa gate, with strict status
  checks, resolved conversations, linear history, and no deletion/force pushes.
  Zero extra approving reviewers are required for this single-maintainer project.
- Workflow default permissions are read-only; workflows cannot approve PRs.
- The release environment accepts deployments from main only, without an
  additional manual reviewer gate. main protection is to match qa before release.
- The OpenSSF Scorecard workflow ran successfully on dev. It publishes against
  the current default branch and runs weekly or by manual dispatch.

The REUSE badge service can scan the public repository. Local REUSE lint passes;
the badge's refresh schedule is external. Dev, QA, Scorecard and REUSE badges
link to their actual services. A missing release or questionnaire ID is not
represented as a completed badge.

## First crates.io publication

The first release candidate is h8-asm 0.5.0. The maintainer has supplied a
local publishing credential for the authenticated bootstrap. It is saved in
Cargo's local credential store and the GitHub release environment secret
CARGO_REGISTRY_TOKEN. The [crates.io Trusted Publishing rollout](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/)
requires the first version to be published manually before attaching a trusted
publisher. Prepare and verify the final crate first, authenticate with cargo
login locally, and publish the reviewed 0.5.0 artifact. Never put the token in
a chat, issue, commit or log.

The supplied token cannot manage Trusted Publishing: the configuration API
returned HTTP 403 (insufficient permissions). Publication uses the environment
token; OIDC is not claimed configured. After the first publication, use an owner
session or a token with Trusted Publishing permission to add this GitHub publisher:

| Field | Value |
|---|---|
| Repository owner | MattJackson |
| Repository | h8-asm |
| Workflow | release.yml |
| Environment | release |

After registering the publisher, set the release environment variable
CRATES_IO_TRUSTED_PUBLISHING=true to select OIDC. Until that setting is enabled,
the workflow uses the environment token, including for the first publication.

The release workflow re-runs full QA and checks the registry, git tag and
GitHub Release before taking any missing step. Completed releases are no-ops;
incomplete tagged releases must resume from their original commit. The package
must byte-match the immutable registry artifact before signing or attestation. It can finish tagging and
release creation after the authenticated bootstrap without republishing the
crate. Its dry-run mode performs verification without publishing. A non-main
dispatch runs verification only. No successful publication is claimed yet.

## Codecov

Codecov is active. The first OIDC upload succeeded in QA run 36263344962
on 2026-09-26 for commit 0e0cb921977a14bae0117a65c65d5716b56b5933.
Its processed report shows 100% coverage (2,801/2,801 Codecov-counted lines);
these service counts are distinct from LLVM's source line/region/function
counts. The public API confirms active=true and activated=true.

Uploads use the supported [Codecov OIDC flow](https://github.com/codecov/codecov-action#using-oidc),
with id-token permission limited to the coverage job and its reusable-workflow
caller. CODECOV_TOKEN remains an optional alternative. The source coverage gate
runs first; an attempted upload must succeed. The README badge links to the
verified service and currently displays dev while main is being established.

## OpenSSF Best Practices

Registration and questionnaire submission require the maintainer's account
at https://www.bestpractices.dev. Use the actual repository URL and the evidence
in openssf-best-practices.md. That document is an evidence checklist, not a
completed 67-answer questionnaire or a claim of a passing badge. Personal
self-assessments and unmet criteria must be answered honestly.

Record the real project ID and completion state after submission, then add the
assigned badge. No project ID or passing status has been invented. This is
separate from the already completed Scorecard run.

## Remaining authenticated access

GitHub setup is accessible through the authenticated CLI. A local token is
available for crates.io first publication; Best Practices registration still
needs its authenticated account. A browser integration was suggested for this work but is not
confirmed connected. Continue implementation and verification independently;
these account steps must not be confused with requests for routine coding
permission.
