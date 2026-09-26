# Independent encoding checks

Renesas' manuals under `spec/` are the authority. GNU binutils is an
independent implementation used to check the crate's interpretation.
These checks cover the implemented subsets; they do not establish complete
H8 family conformance.

## Reproduce

A C/C++ compiler, make, curl, tar with xz support, and shasum are needed to
build the oracle. The Rust library itself has no external dependencies.
From the repository root:

```sh
scripts/build-binutils.sh
scripts/conformance.sh
```

The build script downloads binutils 2.47 from GNU, verifies the archive's
pinned SHA-256, and builds `h8300-elf` as, ld, objcopy, and objdump under
`target/h8-binutils/install/bin`. `H8_BUILD_JOBS` controls build parallelism
(default 2). `H8_BINUTILS_ARCHIVE` may name an existing copy of the same
archive; the checksum check still applies. An optional first argument to the
build script chooses another build root; an optional first argument to the
conformance script chooses another directory of installed tools.

The tests remain ignored in the normal Rust test suite because they require
external tools. The conformance script explicitly selects all six oracle
test binaries and runs them with `--ignored`: a missing tool or failed subprocess
fails the run. QA runs that script as a required job. This explicit invocation
replaces the proposed environment-variable opt-in in the kickoff; there is
no successful skip path in the conformance job.

## Assemble back

The stronger check is:

```text
original bytes -> typed decode -> Renesas assembly -> GNU as -> GNU ld
               -> GNU objcopy .text -> compare with original bytes
```

The crate's encoder must also reproduce the original bytes before the
external tools run. Translation changes only `$` to `.` (the current
location counter) and `H'` to `0x` (hexadecimal constants). Immediate and
branch width suffixes preserve the exact encoding. Linking is required:
branch relocations in an unlinked object are not final instruction bytes.

The local 2026-09-26 run with GNU Binutils 2.47.20260726 reproduced:

| Probe | Exact-byte cases |
|---|---:|
| Every supported H8/300 two-byte instruction | 2,692 |
| Every supported H8SX two-byte instruction | 39,297 |
| Selected H8SX branches, jumps/calls and absolute byte moves | 140 |
| H8SX word-immediate boundaries, every register and seven operations | 560 |
| H8SX full-width long-immediate boundaries, every ER register and seven operations | 504 |

The two-byte checks enumerate all 65,536 words and select those accepted by
the semantic decoder. The longer cases are structured samples, not exhaustive
four- or six-byte censuses. H8SX assembly uses `.h8300sx` and the linker
emulation `h8300sxelf`; the test uses `Mode::Maximum` in the Rust APIs. This
does not establish that GNU expresses every H8SX operating mode.

## First-word boundary probes

Two further tests compare instruction lengths for every first word on all
five targets with a zero-filled suffix. Each candidate has a 16-byte slot.
They check the start of each slot, not instructions in its padding.

| Target | Matching recognized lengths | Both reject | GNU only |
|---|---:|---:|---:|
| H8/300 | 51,854 | 7,357 | 6,325 |
| H8/300H | 55,713 | 7,357 | 2,466 |
| H8S/2000 | 56,065 | 7,357 | 2,114 |
| H8S/2600 | 56,098 | 7,357 | 2,081 |
| H8SX | 57,677 | 7,357 | 502 |

There were no crate-only decodes or accepted-length disagreements. GNU-only
patterns are reported, not silently counted as agreement. Some are aliases
that violate fixed bits in the manual; some expose implementation gaps or
GNU accepting later instructions in an earlier-core mode. See
[`../spec/H8-ISA.md`](../spec/H8-ISA.md) for examples. A complete, manually
reviewed divergence allow-list remains outstanding.

A zero suffix is only one completion of a prefix. These counts cannot prove
that a rejected first word is undefined, or that an accepted prefix handles
all its valid continuations. The four-billion-pattern sweeps from the
kickoff have not been run.


## Internal operand audits

Separate from the GNU checks, run:

```sh
cargo test --release --test sx_operand_audit -- --ignored --nocapture
```

This enumerates all 7,340,032 H8SX word-immediate patterns and 4,456,448
16-bit branch-field/mode cases, checking typed semantics, recognized length,
encoder output, and odd-displacement refusal. Both passed locally. QA runs
these audits across its OS/MSRV matrix. They establish agreement inside the
crate and do not replace the external assemble-back checks above.


`tests/binutils_planning.rs` adds independent assembly of mode-appropriate
H8SX detour site/trampoline bytes (including `JMP @aa:32`) and a label layout
requiring repeated short-to-long widening. These cases pass through the same
as/link/objcopy byte comparison and are mandatory in the conformance script.

## Shared legacy semantic codec

The shared vocabulary retains operand widths, control registers, register
groups, and the documented long-displacement store alias. These probes use
the shared decoder, renderer and encoder, rather than the original narrower
H8/300 API.

| Target | Two-byte exact | Extended exact | Store alias choices | GNU base-jump divergences | Disassembly-only restricted groups |
|---|---:|---:|---:|---:|---:|
| H8/300 | 51,380 | 2,245 | 0 | 4 | 0 |
| H8/300H | 54,344 | 7,770 | 256 | 0 | 0 |
| H8S/2000 | 54,696 | 9,630 | 320 | 0 | 4 |
| H8S/2600 | 54,729 | 9,694 | 320 | 0 | 4 |

The extended corpus includes every first word with four suffix patterns,
every second word for 19 representative prefixes, displacement boundaries,
and all second opcodes for absolute memory-bit operations. It is deduplicated
before assembly. This is still not exhaustive four-byte enumeration.

The H8/300 assembler uses GNU's default operand mode (no directive); its ELF
machine header nevertheless defaults to h8300h, requiring the h8300helf
linker emulation. H8/300H uses .h8300h/h8300helf; H8S uses
.h8300s/h8300self.

Three differences are classified explicitly in the executable check:

- Long-displacement MOV.L stores permit two values of bit 7 in byte 4
  (H8S REJ09B0139 §2.4 Table 2.2 note 2). GNU chooses the set bit.
  The shared codec preserves the original alias; every other byte must match.
- GNU sign-extends upper-half H8/300 absolute JMP/JSR addresses into the
  reserved high byte. ADE-602-025 fixes that byte to zero. The codec retains
  zero; the four matching divergences are reported separately.
- The generic H8S software manual lists ER6–ER7 and ER4–ER7 LDM/STM groups,
  while product hardware manuals prohibit using ER7 (for example
  [H8S/2169 §2.10.2](https://www.renesas.com/en/document/mah/h8s2169-f-ztat-h8s2149-f-ztat-hardware-manual)).
  GNU refuses their assembly. Four cases per H8S core therefore undergo
  independent disassembly comparison only. They are never counted as exact
  assemble-back successes. The generic codec recognizes their bit patterns;
  this does not establish that they may be executed on a particular chip.

This audit corrected H8S register-group starts: groups of two begin at even
registers; groups of three/four begin at ER0 or ER4. Arbitrary consecutive
groups belong to H8SX. Legacy 16-bit branches also now reject odd
displacements, consistently with their instruction alignment.

## H8SX full-table boundary grammar

The conformance script also checks reproducibility of the generated runtime
table and runs all 16,986 raw manual-row probes. All differences are pinned
by classification, with no unresolved category accepted. Two inferred AND
cases and two corrected ADD cases additionally assemble back exactly.
See [the row intake and constraints](H8SX-TABLE.md). Semantic assembly of all
H8SX rows remains outstanding.

## Shared H8SX semantics

The mandatory binutils_sx_shared tests independently assemble all 56,080
supported two-byte instructions exactly. Five constrained completions per
8,493 manual rows (zero, one, and three asymmetric patterns) yield 42,345
unique instructions: 42,315 exact assemble-back results, 12 recorded
alternative encodings whose complete typed operands agree, and 18 cases
checked by independent GNU disassembly instead. All three counts are pinned.

The disassembly-only cases are explicit GNU 2.47 limitations:

- Six zero five-bit SHLL/SHLR counts: Renesas §2.2.99 [8] permits zero,
  but gas constant_fits_size requires L_5 >= 1. GNU objdump decodes the
  raw bytes to the expected zero-count operation.
- Twelve MOVA forms: gas get_mova_operands compares source/destination
  register numbers masked with 7, then incorrectly selects the compact
  form for RnH (instead of RnL) or En (instead of Rn). The full four-bit
  register map on manual §2.4 p.889 and §2.2.66 preserve the distinction.
  GNU objdump independently agrees with the full form's register identity,
  displacement and destination. These are never counted as assemble-back.

The Renesas extended-vector width suffix :7 is removed only for GNU syntax;
the physical vector address is preserved. GNU's odd-offset warning is
accepted only for JMP/JSR absolute fields or MOVSD.B and only when the
original operand is actually odd. The manual's PC description (§1.5.2)
ignores bit zero during fetch; the shared codec retains the encoded field,
while control-flow analysis resolves the effective even address.

The earlier narrow API oracle remains mandatory. No round-trip or sampled
row census substitutes for the full four-byte and operand sweeps.


The longer-field corpus varies every register/small field independently
through all values, and each larger field through zero, one, two, signed
midpoint boundaries and unsigned maximum boundaries. Its 163,471 valid
samples (>4 bytes) yield 163,435 exact GNU assemblies, 18 verified alternative
forms, and 18 MOVA disassembly-only cases. These counts are pinned separately
from the five-witness row corpus; the corpora overlap and must not be summed
as unique instruction coverage.

## Reviewed H8SX reverse census

The zero-suffix first-word test pins the exact 502 GNU-only triples (word,
length, mnemonic) in tests/data/sx_reverse.tsv. Every entry records a reason
and source citation. The test fails if the set changes, even if its total
stays 502. These are refusals justified against the manual, not instructions
quietly counted as supported:

| GNU-only group | Count | Manual restriction |
|---|---:|---|
| STMAC/LDMAC | 96 | §2.4 pp.731/832 fixes register bit 3 and opcode bit 7 to zero |
| STC/LDC VBR/SBR | 32 | Long-register field is three bits; bit 3 is zero |
| ADDS/SUBS | 48 | ERn field is three bits; bit 3 is zero |
| EXTU.L/EXTS.L #2 | 16 | §2.4 pp.728–729 fixes bit 3 to zero |
| RTS/L, RTE/L | 74 | §2.2.94/96: one to four serial registers, no underflow; §2.4 p.813 reserves other group counts |
| BSR d:8 | 128 | §2.2.30 and §1.8.8 require an even displacement |
| TRAPA | 12 | §2.4 p.861 provides two vector bits and fixes bits 7–6 to zero |
| JSR selector | 95 | §2.4 p.730 requires register low nibble zero, absolute-32 selector 08, or vector bit 7 set |
| BPT 7aff | 1 | GNU h8300.h O_BPT debugger extension has no row in the pinned Renesas table |

The JSR set excludes the valid PC-indexed BSR selectors occupying the same
first-byte space. RTS/L and RTE/L also exclude the valid plain return words.
The first-word test remains a fixed-zero-extension comparison; it is not
an exhaustive reverse comparison of every longer instruction.

## Operating-mode audit

GNU 2.47 gas exposes .h8300h/.h8300hn, .h8300s/.h8300sn and
.h8300sx/.h8300sxn. Its default instruction mode is H8/300; there is no
.h8300 pseudo-op in tc-h8300.c's md_pseudo_table. Normal-mode directives set
Nmode; the unsuffixed forms select the extended non-normal architecture.
There is no distinct middle- or maximum-mode directive or machine selector.
Therefore GNU checks encoding, not the library's middle/maximum address rules.

Independent normal-mode runs assemble all 54,344 H8/300H, 54,696 H8S/2000,
54,729 H8S/2600 and 56,080 H8SX supported two-byte instructions exactly.
H8SX's normal-mode five-witness row corpus has the same 42,315 exact / 12
alternative / 18 disassembly-only counts as its non-normal run. In particular,
extended vectors render physical addresses 0x100–0x1fe in normal mode and
0x200–0x3fc otherwise. These runs are mandatory oracle tests.

Program-address width, vector-entry width, wrapping and unsupported context
combinations are checked against the manuals in the library tests. No GNU
run should be described as independent validation of stacked PC semantics
or maximum-mode hardware execution.

An additional inherited-field corpus applies the longer H8SX field witnesses
to legacy targets that recognize the corresponding encodings. It provides
280 distinct H8/300H and 1,274 each H8S/2000 and H8S/2600 exact GNU assemblies,
with those counts pinned. Normal/advanced typed round trips cover the same
cases. These supplement the earlier legacy-specific displacement/register
probes; H8/300 has no instruction longer than four bytes.
