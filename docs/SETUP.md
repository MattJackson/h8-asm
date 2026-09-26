# Release and service setup

Use the maintainer identity Matthew Jackson, matthew@pq.io. This page records
real configuration and the remaining account steps. The implementation's
coverage and conformance gates do not depend on a badge claiming completion.

## GitHub

The public repository is https://github.com/MattJackson/h8-asm. Sources are
developed on dev; qa runs the full verification workflow. main is the default
branch. Promotion follows dev → qa → main.

Completed configuration:

- Private vulnerability reporting is enabled; SECURITY.md gives both the
  private advisory route and the maintainer email.
- dev rejects deletion and force pushes and requires linear history, including
  for administrators.
- qa requires the successful dev check (`fmt, clippy, test`), linear history,
  and no deletion/force pushes. Direct fast-forward promotion preserves the SHA.
- An active main ruleset requires the Actions `qa gate` check, linear history,
  and no deletion/force pushes. No actor has a ruleset bypass.
- Workflow default permissions are read-only; workflows cannot approve PRs.
- The release environment accepts deployments from main only, without an
  additional manual reviewer gate.
- The OpenSSF Scorecard workflow ran successfully on dev. It publishes against
  the current default branch and runs weekly or by manual dispatch.

promote.yml advances the exact current successful dev commit to qa and dispatches
QA, then advances the exact successful qa commit to main and dispatches release.
It validates repository/workflow/event/ref identity, rejects non-fast-forwards,
and never checks out code in its privileged job. Explicit dispatch is necessary
because a GITHUB_TOKEN ref update does not itself trigger another workflow.
The dev → qa promotion of commit 15a15a0 succeeded in run 36266036464; full
QA passed in run 36266043197. GitHub did not start the QA completion callback,
so the same CI promotion was dispatched with destination=main (run
36267120070). Its API checks validated QA and created main at exactly that SHA.
The initial ruleset rejected branch creation despite the successful check;
the branch-creation exception was enabled for this bootstrap and immediately
restored to false after CI created main. main was then set as the default.
If a completion callback does not arrive, `gh workflow run promote.yml
--ref main -f destination=main` resumes through the same validation; it does
not push a branch locally or bypass failed QA.

Local REUSE lint passes. The REUSE API accepted registration on 2026-09-26
(HTTP 202) and sent a confirmation email to the maintainer. The external badge
remains pending that confirmation and its service scan; it is not claimed
compliant until the service reports it. Dev, QA, Scorecard and REUSE badges
link to their actual services.

## First crates.io publication

The first release is h8-asm 0.5.0, published on 2026-09-26. The maintainer supplied a
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
dispatch runs verification only. Release run 36267228252 completed successfully
on commit 15a15a023312dd1927f96c18895dcd12e1ef6a01, published 0.5.0, created
tag v0.5.0, and uploaded all five signed/attested release assets. Independent
download verification passed for the exact release workflow identity. The
Published rustdoc is available at https://docs.rs/h8-asm/0.5.0/h8_asm/. The
registry and GitHub archive both have SHA-256
`07f60b20bbdeac16166434ca88ab1ea59438a0d5fcc79399c5efd6f7469c5a93`.

## Verifying release artifacts

Each completed release includes the crate archive, a Sigstore bundle,
detached signature/certificate, and SLSA provenance. For version 0.5.0:

```sh
gh release download v0.5.0 --repo MattJackson/h8-asm --pattern 'h8-asm-0.5.0.crate*'
gh attestation verify h8-asm-0.5.0.crate --repo MattJackson/h8-asm
cosign verify-blob --bundle h8-asm-0.5.0.crate.cosign.bundle \
  --certificate-identity-regexp '^https://github.com/MattJackson/h8-asm/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  h8-asm-0.5.0.crate
```

Keyless verification checks the certificate identity and issuer in the bundle;
there is no long-lived project signing key on the download server. Compare the
crate archive with the registry download when establishing artifact identity.
Annotated git tags are not themselves claimed to be cryptographically signed.

## Codecov

Codecov is active. The first OIDC upload succeeded in QA run 36263344962
on 2026-09-26 for commit 0e0cb921977a14bae0117a65c65d5716b56b5933.
Its processed report shows 100% coverage (2,801/2,801 Codecov-counted lines);
these service counts are distinct from LLVM's source line/region/function
counts. The public API confirms active=true and activated=true.

Uploads now use the GitHub repository secret CODECOV_TOKEN, as requested by
the maintainer. Coverage OIDC is disabled. The release workflow forwards only
this named secret to its reusable QA workflow. The source gate runs first and
an attempted upload must succeed. The initial OIDC success above records the
activation history. The token-authenticated upload succeeded in coverage job
108468708345 of QA run 36265297104.

## OpenSSF Best Practices

The maintainer registered [project 14963](https://www.bestpractices.dev/projects/14963)
and authorized direct form submission. The target is 198% tiered (passing
100%, silver 98%), matching the reference project. Use the actual repository
URL and the evidence in openssf-best-practices.md. After the maintainer
confirmed both personal security-knowledge declarations on 2026-09-26, both
forms were submitted and the service awarded the passing badge: 100% passing,
98% silver, 198% tiered after independently verified release-signing evidence
was submitted. Access continuity remains Unmet because no alternate
administrator or credential succession arrangement is established. This is
separate from the already completed Scorecard run.

## Remaining authenticated access

GitHub setup is accessible through the authenticated CLI. A local token is
available for crates.io first publication. The Best Practices forms are saved at the target score. Trusted Publishing
configuration still requires an owner session or a token with that scope.
