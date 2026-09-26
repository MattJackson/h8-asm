# Security policy

h8-asm reads and modifies caller-owned flat firmware images. It does not
execute instructions or access devices, networks or files. Incorrect decoding,
encoding, branch relocation or patch placement can still produce unsafe
firmware. Treat silent wrong output, unchecked bounds and non-atomic failure
as security-relevant correctness defects.

Report sensitive issues through [GitHub private vulnerability reporting](https://github.com/MattJackson/h8-asm/security/advisories/new) or matthew@pq.io. Include the version or
commit, target and operating mode, exact input bytes or typed instruction,
API call, expected result and relevant Renesas manual section. A minimal
failing test is particularly useful. Do not publish exploitable firmware
or third-party secrets in an issue.

The current development version is the supported version until a release
exists. After release, fixes target the latest published minor series;
older versions may require upgrading. No response-time or hardware-validation
guarantee is made. Confirm the release status in CHANGELOG.md and crates.io.

The maintainer triages private reports, reproduces the issue with the reporter,
and prepares a regression test and fix. Confirmed security fixes are published
in a new crate version with a GitHub Security Advisory and a CHANGELOG entry
identifying the affected and fixed versions. Disclosure timing is coordinated
with the reporter; reporters receive credit unless they request anonymity.
Ordinary correctness reports remain public issues when disclosure is safe.

The project also checks its build and release environment with Zizmor, which
looks for workflow command injection, unsafe credential handling and related
GitHub Actions vulnerabilities. This supplements Rust/Clippy analysis; it is
not a vulnerability scan of firmware produced by library callers.

A valid encoding is not evidence that a patch is safe for a particular
product. Callers must supply the correct target, mode, instruction boundaries,
image layout and hook behavior. Product hardware restrictions can be stricter
than the generic software manual, including H8S multi-register operations.
See docs/ASSURANCE_CASE.md and docs/PATCHING.md for the verification boundary.
