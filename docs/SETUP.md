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

After the first publication, open the crate's Trusted Publishing settings and
add this GitHub publisher:

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
GitHub Release before taking any missing step. It can finish tagging and
release creation after the authenticated bootstrap without republishing the
crate. Its dry-run mode performs verification without publishing. A non-main
dispatch runs verification only. No successful publication is claimed yet.

## Codecov

The repository is visible through Codecov's public API but was inactive before
its first upload. QA now attempts authenticated uploads using the supported
[Codecov OIDC flow](https://github.com/codecov/codecov-action#using-oidc), with
id-token permission limited to the coverage job and its reusable-workflow
caller. An explicit CODECOV_TOKEN secret remains an optional alternative.
The first upload still needs verification; do not call the service active
until it succeeds.

If Codecov requires account activation, sign in at
https://app.codecov.io/gh/MattJackson/h8-asm and enable the GitHub app for this
repository. A service authentication failure must be reported and corrected;
it is not silently treated as a passed upload. Add the coverage badge only
after a successful report is visible. The 100% source gate remains mandatory
before upload.

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
