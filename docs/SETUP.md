# One-time manual setup

Everything in this repository that a workflow cannot do for itself, in the order
it should be done. Each step is a click-through on someone else's website, which
is why none of it is automated: each one gates account creation or an
organisation setting behind an interactive login.

Nothing here blocks development. `dev` and `qa` are fully green without any of
it; step 1 blocks *releasing*, and steps 2 and 3 only add badges.

Register everything as **Matthew Jackson, matthew@pq.io**.

Everything already in the repository is wired to notice when each step is done
and start working on the next run — no workflow edit follows any of these.

---

## 0. Push the branch first

Several of these steps verify a URL, and the Best Practices questionnaire in
particular cites `blob/main/...` links for `CONTRIBUTING.md`, `REUSE.toml`,
`LICENSES/`, `docs/` and `.github/`. Promote `dev` → `qa` → `main` as usual
before starting step 3, or those links 404 under review.

Two badges in `README.md` go live on their own after the push, with no account
and no action needed:

- **OpenSSF Scorecard** — reads "unknown" until the first run of
  `.github/workflows/scorecard.yml`. That is scheduled weekly (Mondays 06:27
  UTC); to get it immediately, run it once by hand from
  Actions → "scorecard" → "Run workflow". Requires the workflow to exist on the
  default branch first.
- **REUSE** — reads "unknown" until `api.reuse.software` first scans the pushed
  branch. `reuse lint` passes locally today; the badge follows within a day of
  the push, or immediately if you load
  <https://api.reuse.software/info/github.com/MattJackson/h8-asm> once, which
  triggers a scan.

---

## 1. crates.io first publication and Trusted Publishing

As of this build-out, h8-asm has no published version and the local session
has no cargo publishing credential. The first publication requires an
authenticated owner: crates.io's [Trusted Publishing rollout](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/)
requires the first release to be published manually before attaching a
publisher. This bootstrap must not be mistaken for an already configured
OIDC release path. Prepare and verify the final crate first, authenticate
locally with cargo login, publish the approved first version, then configure
the publisher below. Never paste the token into an issue, commit or log.

Without this, `release.yml` has no way to authenticate and the publish step
fails. There is no long-lived token anywhere in this repository by design:
GitHub mints a short-lived OIDC token per run and crates.io exchanges it for a
~30-minute publish token.

1. Sign in to <https://crates.io> as an owner of the `h8-asm` crate.
2. Go to <https://crates.io/crates/h8-asm/settings> → **Trusted Publishing**.
3. **Add** a publisher, choose GitHub, and enter exactly:

   | field | value |
   | --- | --- |
   | Repository owner | `MattJackson` |
   | Repository name | `h8-asm` |
   | Workflow filename | `release.yml` |
   | Environment | `release` |

   The environment is optional on crates.io's side, but fill it in: it is what
   stops a workflow on some other branch or in a fork from minting a token.
4. On GitHub: **Settings → Environments**. The `release` environment is created
   automatically on the first run, but creating it by hand lets you add
   *Required reviewers* (yourself), so every publish waits for a click, and a
   deployment branch rule limiting it to `main`. Recommended.

The long-form version of this, including the fallback to a
`CARGO_REGISTRY_TOKEN` secret if you ever want one, is in the header comment of
`.github/workflows/release.yml`.

---

## 2. Codecov — activate, then add one secret

The coverage **gate** does not depend on this. `qa.yml` enforces a hard 100%
line/region/function floor locally and fails the build below it whether or not
Codecov exists. Codecov adds the history and the per-line diff view, and a badge.

1. Sign in at <https://app.codecov.io> with GitHub (authorise the `MattJackson`
   account) and activate the **h8-asm** repository.
2. Copy the repository upload token from
   <https://app.codecov.io/gh/MattJackson/h8-asm/settings>.
3. On GitHub: **Settings → Secrets and variables → Actions → New repository
   secret**, named exactly `CODECOV_TOKEN`, with that value.
4. Uncomment the codecov badge in `README.md` after the first successful upload
   (it is sitting in an HTML comment right below the badge block, with a note
   saying the same thing).

That is the whole change. The upload step already exists in `qa.yml`'s coverage
job: it is guarded by `if: env.CODECOV_TOKEN != ''`, so today it skips itself
and writes a note in the run summary explaining why, and the moment the secret
exists it starts uploading. `release.yml` passes the same secret through to the
called `qa.yml`, so pushes to `main` upload too — which matters, because the
badge reports the default branch.

Deliberately *not* used: tokenless upload (Codecov has been tightening it and it
is no longer dependable for public repos) and `continue-on-error` (so that "not
configured yet" and "failed and was ignored" never look alike in a run log).

---

## 3. OpenSSF Best Practices — submit the questionnaire

The answers are already written against this repository in
[`openssf-best-practices.md`](openssf-best-practices.md) — 67 passing-level
criteria, each with the answer, a one-line justification and its URL evidence.
It is meant to be pasted, not composed.

**Before submitting, close the one blocker.** `vulnerability_report_process` is
a MUST and is currently Unmet: there is no published process for reporting a
vulnerability. The fastest fix is **Settings → Code security → Private
vulnerability reporting → Enable**, which publishes a reporting route without
committing anything and also satisfies `vulnerability_report_private`. The
alternative — a `.github/SECURITY.md`, with a draft ready to paste — is in that
document under the same criterion. It was left for you rather than committed
because it commits the project to a contact route and a response time, and that
is not a decision to make on someone's behalf.

Then:

1. Sign in at <https://www.bestpractices.dev> with GitHub.
2. **Submit a project**, with project URL and repository URL both
   `https://github.com/MattJackson/h8-asm`.
3. Work down `openssf-best-practices.md` — it follows the form's own order and
   grouping. Read the two entries marked *maintainer to confirm*
   (`know_secure_design`, `know_common_errors`): they are self-assessments about
   your own knowledge, and the evidence is assembled but the claim is yours.
   Three answers are marked `[JUDGEMENT]` with the reasoning for and against.
4. On submission the site assigns a numeric project ID. Add the badge to
   `README.md`, replacing `NNNNN` with that ID:

   ```markdown
   [![OpenSSF Best Practices](https://www.bestpractices.dev/projects/NNNNN/badge)](https://www.bestpractices.dev/projects/NNNNN)
   ```

   There is an HTML comment in the badge block saying the same, so it is
   findable from the README alone.

---

## Optional, and worth considering

Not required by anything above, but each raises the Scorecard score and two of
them close criteria noted as Unmet in the questionnaire:

- **Branch protection on `main`** (currently none). Require the `qa gate` check
  and require a pull request. Scorecard's Branch-Protection check is one of the
  heavier ones, and `qa.yml`'s `gate` job exists precisely to be the single
  required check.
- **Fuzzing the decoders** — `cargo-fuzz` over `decode_bl`, `decode_b_wide`,
  `decode_b_cond`, `find_bl_sites` and `CommandTable::walk`, which are the
  functions that read attacker-influenced firmware bytes. Closes
  `dynamic_analysis`.
- **A CodeQL job** (`language: rust`) in the qa gate. Closes
  `static_analysis_common_vulnerabilities` and removes any argument about
  whether Clippy counts as static analysis.
- **Dependabot security updates** are disabled on the repository. Harmless
  today — the crate has zero dependencies — and `.github/dependabot.yml` already
  keeps the GitHub Actions pins fresh, which is the only third-party code here.


## Recorded setup progress (2026-09-26)

- Public GitHub repository created at MattJackson/h8-asm; origin configured.
- Development sources pushed to dev, which is the temporary default branch.
- Private vulnerability reporting enabled. Workflow default permissions are
  read-only and workflow review approval is disabled.
- dev rejects force pushes and deletion and requires linear history, including
  for administrators. It allows ordinary development pushes.
- qa/main promotion protection, final release and third-party account setup
  remain in progress. Do not infer activation from a README badge placeholder.
- Local packaging, MSRV, source coverage and REUSE checks pass; hosted checks
  and mutation audit are tracked separately in ROADMAP.md.

Publishing/account authentication is the remaining external prerequisite.
The browser integration suggested during setup is not confirmed connected.
