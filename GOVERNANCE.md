# Governance

Matthew Jackson (MattJackson on GitHub, matthew@pq.io) maintains h8-asm and
makes final decisions on scope, review, releases and security reports. There
is no foundation or steering committee. Contributions and ordinary design
discussion take place through issues and pull requests.

Correctness, cited encoding rules, conservative refusals, safe Rust and
reproducible validation take priority over accepting more input. Changes to
the MSRV, dependency policy, public encoding behavior or verification gates
require an explicit rationale and documentation.

The release path is dev → qa → main, with automatic fast-forward promotion
after each required gate and full QA re-verification before publication.
GitHub and crates.io access belongs to the maintainer; access
must not be shared through repository files or CI logs. See docs/SETUP.md for
the account configuration and its actual completion status.

Security reports and conduct concerns go to the maintainer using SECURITY.md
and CODE_OF_CONDUCT.md. If maintenance stops, users may fork under the MIT
license; there is no automatic transfer of credentials or package ownership.
Any future co-maintainer arrangement must update this document to identify
who can approve changes, release packages and handle private reports.
The bus factor is currently one. There is no verified alternate administrator
or credential succession arrangement, so continuity within one week of losing
the sole maintainer is not claimed. Public source and an MIT license permit a
fork but do not grant control of this repository or the existing crate name.
