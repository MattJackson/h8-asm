# h8-asm kickoff

A working note for starting the build-out. It is not documentation. Delete it,
or fold what is left into `ROADMAP.md`, once phase 3 lands.

## What this is

A decoder, disassembler, encoder, relocator and detour installer for the
**whole Renesas H8 line** (H8/300, H8/300H, H8S/2000, H8S/2600, H8SX), built to
the same contract as `../thumb-asm`:

- It works on a flat image, where offset == address.
- Emitting wrong bytes should be impossible, not merely unlikely, because the
  output gets flashed to hardware.
- Every encoding claim cites a Renesas manual section.
- It refuses rather than guesses.
- Zero dependencies, `forbid(unsafe)`, MSRV 1.58, 100% coverage gate.

thumb-asm is the **template for the method and the scaffolding, not for the
code**. Read these before starting; they are the design being copied:

- `../thumb-asm/README.md`, the sections "How it is checked" and "Design, and what this is not"
- `../thumb-asm/docs/CONFORMANCE.md` (the assemble-back oracle loop, and why text diffing was rejected)
- `../thumb-asm/docs/ASSURANCE_CASE.md`
- `../thumb-asm/spec/THUMB-ISA.md` (the shape of the coverage map to reproduce)
- `../thumb-asm/src/isa/insn.rs` and `mod.rs` (the shared `Insn` vocabulary and the verified-encode dispatch)

## What's already here

| Path | State |
|---|---|
| `spec/*.pdf` + `*.txt` | All four CPU software manuals plus Renesas' assembler manual, with pypdf dumps. Section map and grep recipes are in `spec/README.md`. |
| `src/lib.rs` | `Target`, `Mode` and `Target::supports`, with the mode table pinned by a test. Nothing decodes. |
| `.github/` | dev/qa/release/scorecard workflows and dependabot, copied from thumb-asm with the name changed. The example-run steps and the LLVM `conformance` job were **removed** (see phase 4). |
| `REUSE.toml`, `LICENSES/` | MIT for our files; `LicenseRef-Renesas-Documentation` for `spec/*.pdf|txt`. |
| `Cargo.toml` | `exclude` keeps `spec/` and this file out of the `.crate`. |
| `CODE_OF_CONDUCT.md`, `docs/SETUP.md` | Copied as-is (name changed). |

**Deliberately not copied yet:** `CONTRIBUTING.md`, `SECURITY.md`,
`GOVERNANCE.md`, `ROADMAP.md`, `docs/ASSURANCE_CASE.md`, `docs/CONFORMANCE.md`,
`docs/ENCODING-STABILITY.md` and `docs/openssf-best-practices.md`. Each of them
makes specific claims (LLVM sweeps, probe counts, allow-list sizes, the OpenSSF
project ID) that are false for this repo until the work exists. Port each one
in the phase that makes its claims true, rewriting Arm→Renesas and
LLVM→binutils as you go, rather than copying and "fixing later".

**Not done, and needs your call:**

- The first local commit is on `dev`.
- There is no GitHub repo yet. Create `MattJackson/h8-asm`, push, set up branch
  protection like thumb-asm's, then work through `docs/SETUP.md` (trusted
  publishing, codecov, OpenSSF).
- The crate name `h8-asm` was free on crates.io on 2026-09-25. Consider an early
  `0.0.x` placeholder publish to hold it.

## Facts that shape the design (checked against the manuals)

- **Big-endian.** Instructions are built from 2-byte words (H8SX §1.7.3), and
  the `EA` extension is 8, 16 or 32 bits. thumb-asm's `read_u16`/`read_u32`,
  `Needle::Word` and the free-space finders all assume little-endian, so port
  them with endianness fixed to big, not copied.
- **Variable length.** H8/300 instructions are always 2 or 4 bytes. On later
  cores they are longer: `01xx`/`78xx` prefixes, and H8SX's `@(d:32,ERn)` and
  `@aa:32` forms on both operands. **Establish the exact maximum length from
  H8SX §2.4 in phase 1**; everything downstream depends on it. There is no
  one-halfword length rule like Thumb's, so the length decoder is the first
  real code to write.
- **Operating mode changes meaning** (§1.2): normal = 64 KB; middle = 16 MB
  program and 64 KB data; advanced = 16 MB program and 4 GB data; maximum =
  4 GB. It sets branch-address width, `@@aa:8` vector-entry width and stacked
  `PC` size. `Target` × `Mode` is therefore the decode context, in the way
  `Target` alone is in thumb-asm.
- **PC-relative forms (H8SX)** (§1.8.8–1.8.9): `Bcc`/`BSR`/`BRA/S` with d:8 or
  d:16, and `@(RnL.B|Rn.W|ERn.L, PC)` indexed branches. Those are what
  relocation has to rewrite. There are **no literal pools**, so thumb-asm's
  hardest relocation trap (`Align(PC,4)`) does not exist here.
- **`BRA/S` has a delay slot** (§2.2.24). A detour must never split a
  branch from its delay slot, and relocation must move them as a pair. This is
  the analogue of thumb-asm's `IT`-block refusal.
- **Memory-indirect `@@aa:8` and H8SX `@@vec:7`** (§1.8.10–1.8.11) jump
  through a table at the bottom of memory. The instructions are
  position-independent, but they expose where the vector table is, which is
  useful for `analysis`.
- **11 addressing modes** on H8SX (Table 1.12), compared with 8 on H8/300. Most
  of the combinatorial explosion in the encoding tables comes from the H8SX
  general `EA` prefixes.

## Plan

### Phase 1: spec intake and the length decoder

1. Read §1.7–§1.8 and §2.4 of the H8SX manual in full. Extract §2.4 with
   `pdftotext -layout` (see `spec/README.md`), because the pypdf dump garbles
   its columns.
2. Write `spec/H8-ISA.md`, the coverage map in the format of thumb-asm's
   `THUMB-ISA.md`. Organise it by the first-word opcode map (H8S Table 2.3 /
   H8/300H §2.5 / H8/300 Appendix A, extended by H8SX §2.4). Mark which
   generation introduced each row.
3. `isa::insn_len(bytes, Target, Mode) -> Option<usize>`, tested exhaustively
   over all 65,536 first words for every target. Pin the per-target census
   (how many first words start a 2-, 4-, 6-, … byte instruction, and how many
   are undefined) as literals.

### Phase 2: decoder and encoder, one module per opcode-map table

- `Insn`/`Operand`/`Reg` vocabulary first: registers `Rn`/`En`/`RnH`/`RnL`/
  `ERn`, sizes `.B/.W/.L`, and all 11 EA modes as one `Ea` enum.
- Build the generations in order (H8/300, then 300H, then H8S, then H8SX), so
  that each module's per-`Target` tests are written as the target lands.
- Each module decodes, encodes (with dispatch verified by decoding the encoder's
  output again, as in thumb-asm), round-trips, and asserts its hole count as a
  literal.
- Disassembly syntax follows **Renesas' assembler manual** (`spec/H8-assembler.txt`),
  which is the canonical spelling in the role UAL plays for Arm.

### Phase 3: exhaustive sweeps

- Every 2-byte and every 4-byte pattern (4.3G) × each target, decoded at
  an odd-word address. That is about 10× thumb-asm's 402M sweep and fine on the
  EC2 mutation boxes (see the `mutants` skill for the pattern).
- For anything longer than 4 bytes, sweep the operand fields per encoding
  group (all EA-extension values at the boundaries, every register, every
  size).
- Record the result in §4 of `spec/H8-ISA.md` as counts, with the counting
  method.

### Phase 4: independent conformance (GNU binutils)

- The oracle is `h8300-elf-as` / `h8300-elf-objdump`. **No Homebrew formula
  exists**, so build binutils `--target=h8300-elf` from source locally. In CI,
  either do the same (cache it) or use Debian's `binutils-h8300-hms`, after
  checking that it assembles `.h8300sx`.
- Use the same loop as thumb-asm: bytes → our text → `as` → bytes′, then
  assert bytes′ == bytes, with a meaning-only fallback through `objdump` where
  `as` refuses. Then run the reverse census (patterns binutils decodes and we
  reject).
- gas directives select the core and mode: `.h8300` / `.h8300h` / `.h8300s` /
  `.h8300sx`, plus `n` suffixes for normal mode (`.h8300hn` etc.). Check
  whether gas can express H8SX middle and maximum modes at all.
- Expect binutils bugs on H8SX. It is far less exercised than LLVM's Thumb.
  The manual wins, and each divergence gets an allow-list entry citing the
  section.
- Restore the `conformance` job in `qa.yml` (with `H8_ASM_REQUIRE_BINUTILS=1`,
  following the `THUMB_ASM_REQUIRE_LLVM` pattern), then port `CONFORMANCE.md`.
- Optional second oracle: MAME's `src/devices/cpu/h8/` disassembler. Check its
  H8SX coverage first.

### Phase 5: relocate, detour, `Asm`, and the non-ISA helpers

- `relocate`: `Bcc`/`BSR` d:8 → d:16 widening, PC-indexed forms, and a
  `BRA/S` delay-slot pair moved as a unit or refused.
- `detour::tramp`: choose the hook branch per `Mode`. `JMP @aa:24` is 4 bytes
  and covers advanced mode; confirm the maximum-mode form and its length from
  §2.4. Plan fully before writing a byte; leave the image unchanged on error.
- Port `find`/`Needle`/`find_free_space`/`insert`/`write`/`CommandTable`/
  `analysis::{xrefs, reachable, function_start}` from thumb-asm, big-endian.
- `Asm`: a position-independent builder with labels. There is no literal pool,
  so it is simpler than thumb-asm's.

### Phase 6: harden and publish

- Run `cargo-mutants` on EC2, and state survivors honestly in the roadmap as
  thumb-asm does.
- Port `ASSURANCE_CASE.md`, `SECURITY.md`, `CONTRIBUTING.md`, `GOVERNANCE.md`
  and `ENCODING-STABILITY.md`, with every claim now true.
- Complete `docs/SETUP.md` (trusted publishing, codecov, OpenSSF), add the
  README badges, then do the first `0.5.0` release (maintainer override, 2026-09-26) via dev → qa → main.

## Open questions

1. **Consumer.** Is there a specific H8 firmware driving this? Its target and
   mode decide which generation gets depth first, and whether detour matters
   before phase 5.
2. **Shared core with thumb-asm?** The plan is to copy the search/patch helpers,
   which avoids cross-repo coupling. Revisit if the duplication hurts; an
   `Endian` parameter is the seam that would let them be shared.
3. **H8SX product-specific differences.** The manual notes that some
   instructions and states "may differ depending on the product". Decide whether
   `Target` ever needs finer grain than the five cores.

## Resume checkpoint (2026-09-25)

Phase 1 remains in progress. `src/isa/length.rs` recognizes lengths through
H8S; `src/isa/sx_length.rs` recognizes H8SX NOP/SLEEP/RTS/RTE, selected
direct branches (including BRA/S), JMP/JSR absolute, register and vector
forms, register-indirect and selected absolute MOV.B/W rows, all byte-immediate
register and byte absolute-move rows, byte register pairs, selected word/long
register pairs, the seven word- and seven long-immediate register rows, the
RTS/L and RTE/L register-range rows, the MOV.L `0100` prefix, and the ADD.B
32-bit source displacement
family with several destination addressing modes. The full §2.4 table establishes
14 bytes as the H8SX maximum: at most three opcode words and two 32-bit
operand extensions, attained by the ADD.B two-displacement row.
`tests/length.rs` pins zero-suffix first-word censuses for
all five targets and checks modes, truncation, and selected manual prefix,
extension and register boundaries. `spec/H8-ISA.md` records the initial map
and the limits of those counts. No semantic decoder or encoder exists yet.

The first local commit is on `dev`; there is no remote. At this checkpoint,
`cargo test`, `cargo clippy --all-targets -- -D warnings`, and the 100%
line/region/function `cargo llvm-cov` gate passed. The licensed text dumps
retain the whitespace of the source PDFs; `git diff --check` reports that
whitespace on the initial commit, so use it on new source/docs changes rather
than treating the imported dumps as hand-edited text.

Next: finish the H8SX §1.7–1.8 and full §2.4 opcode-map review, then implement
the remaining length recognition and richer prefix
censuses. Extracting PDF pages 653–660 covers only the start of §2.4; the full
table runs through roughly PDF page 907. Do not treat the zero-suffix rejection
counts as undefined-instruction counts. Repository creation/publishing and
phase 2 onward remain outstanding.

Further checkpoint (2026-09-25): H8SX recognition now also covers direct
CCR/EXR byte transfers, CCR immediate operations, register bit operations,
immediate bit operations to byte registers, and three-bit word immediate
ADD/MOV/SUB/CMP, unsigned byte/word multiply and divide register rows, and
the four TRAPA vectors.
The zero-suffix H8SX census is now 9,353 rejected, 54,844 two-byte,
1,217 four-byte, and 122 six-byte first words. Built GNU
binutils 2.47 locally under `/tmp/h8-binutils/build` for `h8300-elf`;
`gas/as-new` and `binutils/objdump` assembled/disassembled representative
H8SX encodings from these groups, including CCR/EXR, bit, and word-immediate
forms. This is a smoke check, not the phase-4 conformance sweep. The build is
temporary and must be reproduced if `/tmp` is cleared. Tests, Clippy, and the
100% coverage gate passed after these additions.

Next checkpoint (2026-09-25): `7axx` H8SX long-register immediates now
distinguish 16-bit (four-byte instruction) from 32-bit (six-byte instruction)
extensions. The `0a`/`0f`/`1a`/`1f` compact long immediates and the `0b`/`1b`
ADDS/SUBS/INC/DEC rows are recognized with manual-defined register widths.
The H8SX zero-suffix census is 8,929 rejected, 55,212 two-byte, 1,273
four-byte, and 122 six-byte first words. An ignored opt-in integration test
now reproduces a complete first-word comparison with binutils 2.47:
57,391 matching recognized lengths, 7,357 rejected by both, 788
binutils-only decodes, and zero accepted-length disagreements. Some
binutils-only decodes are manual-forbidden aliases, so this is a diagnostic
probe rather than a completeness score. See §5 of `spec/H8-ISA.md`.
The `10`–`13` shift and rotate register rows have also been added; the H8SX
`17` NOT/NEG/EXTU/EXTS register rows now use the manual's fixed bit masks.
The H8SX zero-suffix census is now 8,145 rejected, 55,996 two-byte, 1,273 four-byte,
and 122 six-byte first words.

Integration checkpoint (2026-09-25): the H8SX `01` prefix now recognizes
selected LDM/STM, control-register memory/immediate, and MAC forms (460
verified second-word encodings; unchanged zero-suffix first-word census).
`isa::decode::decode` now provides a deliberately narrow H8/300 semantic
decoder for NOP/SLEEP/RTS/RTE, ADD.B/MOV.B register pairs, Bcc d:8, and
BSR d:8. It refuses unsupported encodings and odd branch displacements.
`isa::sx_semantic::decode` covers a similarly narrow H8SX control-flow and
CMP.B immediate subset, with typed signed displacements and 24-bit targets.
The legacy length recognizer now also refuses odd Bcc/BSR d:8 displacements
as required by the H8/300 manual. The updated zero-suffix censuses are in
`spec/H8-ISA.md`. H8SX BSR d:8 and d:16 Bcc/BSR also refuse odd
displacements, while BRA/S retains its distinct odd low-bit opcode. An
ignored binutils probe covers all four legacy targets; after the branch
fix, each has zero crate-only decodes and zero accepted length disagreements
under the zero-suffix probe. The full tests, Clippy,
and 100% line/region/function coverage gate pass. Semantic decode remains
minimal and no encoder, relocator, or detour installer exists yet.

Next integration (2026-09-26): H8/300's semantic subset now has an encoder
and Renesas-style disassembler. All 2,692 decodable two-byte words round-trip
through the encoder, and an opt-in GNU binutils assemble/link/objcopy test
reproduces all 2,692 exact words from rendered assembly. The linker resolves
branch relocations; Renesas `$` is translated to GNU gas `.` in that test.
H8SX length recognition added 02/03 MAC/control/counted-shift forms and
6a18/6a38 absolute byte-immediate arithmetic/logic forms. Its zero-suffix
census is now 8,081 rejected, 56,060 two-byte, 1,273 four-byte, and 122
six-byte first words. The binutils first-word probe now reports 57,455
matching recognized lengths, 7,357 rejected by both, 724 binutils-only,
and no crate-only or accepted-length disagreements. The encoder and
disassembler remain limited to the small H8/300 semantic subset.

Further integration (2026-09-26): the narrow H8SX semantic decoder now has
matching disassembler and encoder support. All 10,369 semantically decoded
two-byte words pass decode/length/encode consistency, and GNU binutils 2.47
assembles their rendered text back to exact bytes. Another 140 four-byte
cases pass the same oracle. The four-byte subset includes branch boundaries,
all Bcc conditions, direct jump/call address extremes, and absolute byte
moves across all byte-register codes. H8SX length recognition additionally
covers selected `7b` EEPMOV and `7c`–`7f` memory-bit forms, which require a
nonzero second opcode word; the zero-suffix first-word census is unchanged.
The semantic and conformance coverage remains narrow relative to the full
H8SX tables.

Planning estimate, made at this checkpoint: 1–3 weeks of focused work for
reliable length recognition across all five targets, and 8–16 weeks for the
full six-phase scope. These are rough effort estimates, not release dates.
H8SX table complexity and independent conformance findings are the largest
unknowns.


Resume checkpoint (2026-09-26, unattended continuation): preserved and finished
the pending CMP.L #xx:32,ERn change. H8SX now semantically decodes, formats,
and encodes all eight byte-immediate rows, all seven word-immediate and
full-width long-immediate rows, and twenty single-word register-pair rows.
The exhaustive two-byte semantic census increased from 10,369 to **39,297**.
Both public subset encoders assert their output decodes to the same typed
instruction and length before returning bytes. This is an internal
consistency check, not a proof against the manual.

Independent GNU assemble-back passes for all 39,297 H8SX two-byte words,
140 previous four-byte cases, and 1,064 additional immediate boundary cases
(560 word, 504 long). All 2,692 H8/300 words still assemble back. The five
target first-word length probes have unchanged counts and no crate-only
acceptances or accepted-length disagreements. `scripts/build-binutils.sh`
reproduces as/ld/objcopy/objdump from a checksum-pinned archive;
`scripts/conformance.sh` runs all four oracle tests without a skip path.
Both scripts were exercised locally from a fresh build. QA now requires a
conformance job. `docs/CONFORMANCE.md` records the exact method and limits.

The resumed tree initially failed the 100% coverage gate. Added missing
CMP.L encode/render/boundary tests, MOV.B rendering and store truncation
checks; removed an impossible match arm in the memory-bit length recognizer.
Rust 1.58 tests and the 100% line/region/function gate pass. No change to
length recognition's accepted language was intended or observed.

Still outstanding: complete H8SX length recognition and manual row map;
semantic support beyond the listed subsets (including H8/300H and H8S);
unified instruction/addressing vocabulary; full phase-3 four-byte/operand
sweeps; reviewed conformance divergence allow-list and mode audit; relocation,
detours, label assembler, search/patch/analysis helpers; mutation audit;
release/governance/security setup and publication. No remote exists at this
checkpoint. **The six-phase project is not complete.** Source coverage and
the subset conformance counts must not be described as 100% ISA coverage.


Additional operand audit at this checkpoint: release-mode enumeration passed
for every word-immediate register/literal combination (7,340,032 patterns)
and all 16-bit Bcc/BSR fields in all four H8SX modes (4,456,448 cases,
including odd displacements that must be refused). The QA OS/MSRV matrix
now requires these two audits. Full cross-target four-byte enumeration is
still outstanding. Final local validation also passed formatting, Clippy,
rustdoc with warnings denied, and verified Cargo packaging; the packaged
file list excludes `spec/`. No release or remote changes were made.


Continuation checkpoint (2026-09-26): the user explicitly requested continuing
to the full scope without pausing. An active long-running goal now tracks the
six-phase completion objective. Do not treat this checkpoint as completion.

Implemented `image` (big-endian Needle/search/checked reads/transactional
write/erased insert/validated command tables), `Asm` (named labels and branch
relaxation), `relocate` (recognized streams, internal relative-target remapping,
mode-width arithmetic and Bcc/BSR widening), `detour::{plan,tramp}` (whole
instruction overwrite, mode-appropriate absolute hooks and jump-back,
erased-space/overlap checks and error atomicity), and `analysis::{xrefs,
reachable,function_start}` (including mode-sized vector entries and explicit
unresolved flow). Public contracts and deliberate refusals are in
`docs/PATCHING.md`.

The relocation tests caught and fixed premature range rejection: an earlier
widening can bring a later external long branch into range, so range failure
must wait for stable layout. Absolute transfers into the moved block are
refused rather than copied into overwritten code. BRA/S and PC-indexed
relocation remain refused. Function lookup uses supplied entries; it does not
guess prologues. These features remain bounded by the incomplete instruction
length recognizer, especially on H8SX.

All new modules pass the 100% line/region/function gate (2,943 regions, 1,754
lines, 83 functions in the full library at this checkpoint). The Rust 1.58
suite passes. A fifth mandatory GNU oracle test confirms advanced/maximum
H8SX hook/trampoline bytes and label-relaxation output byte-for-byte.

Next: finish verification/docs for this increment, then return to the major
remaining ISA work: unified typed operands/instructions, full legacy semantic
coverage, complete H8SX §2.4 row coverage and length handling, and fuller
conformance audits. Mutation hardening and remote/release setup remain pending.
The active completion goal must remain active while this work continues.


Continuation checkpoint: shared legacy semantic codec and oracle (2026-09-26).

Added `isa::{Insn,Operand,Reg,Ea,Size,Encoding}` and shared decode/encode/render.
Recognized legacy forms now carry semantic operands, including memory,
register groups and control registers. The original subset APIs are retained.
Every supported two-byte instruction across four legacy cores assembles back
exactly (215,149 cases). Extended conformance reports 29,339 exact cases,
896 documented store aliases, four GNU base-JMP/JSR divergences, and eight
product-restricted H8S ER7 groups checked by disassembly only. These categories
are deliberately separate. All six oracle test binaries are now mandatory.

This exposed and corrected invalid H8S LDM/STM group starts and odd legacy
16-bit branch acceptance. Adversarial typed operand boundary checks pass.
Full local gate: 4,716 regions, 2,727 lines, 119 functions at 100%; Rust 1.58
all-target suite, Clippy with warnings denied, rustdoc with warnings denied,
and the full GNU conformance script pass. Decoder internal helpers rely on
the already validated opcode language instead of duplicating impossible
refusal paths. The shared codec still refuses H8SX; its separate API remains.

Next primary task: complete the H8SX manual row map, length rules and shared
semantics. Full four-byte sweeps, mutation audit and release setup remain
outstanding. The full six-phase goal remains active; this is an incremental
checkpoint, not project completion.

H8SX row-map work has begun but is not integrated into the library. A temporary
coordinate-based PDF extractor is at /tmp/extract-h8sx.py, reading
/tmp/h8sx-table.xml (pdftotext -bbox-layout PDF pages 653–907) and producing
/tmp/h8sx-rows.json. It identifies 8,493 mnemonic rows. Opcode glyph widths
are derived from the table's bit-column coordinates, not guessed from spacing.
Wrapped mnemonic rows require vertical row spans. Two extraction issues
remain before this can become a reproducible checked-in generator: the final
XORC row currently includes the following paragraph (filter by table font
height, or cap its span); and page 673's first AND.B row has no operand text
in either Poppler or pypdf extraction. Its opcode appears to be the
@(d:32,ERs.L),@(d:32,ERd.L) form, but that inference must be visually/manual
checked and explicitly recorded. One overlapping "00" glyph at page 651 is
one zero bit by coordinates. Page 696's third "15" header is extracted as "1";
the extractor derives that column from its 8/0 headings. No generated row has
yet changed acceptance. Finish extraction validation, field constraints and
independent row probes before replacing or expanding the recognizer.

H8SX intake continuation (2026-09-26): the row extractor is now reproducible
as scripts/extract-h8sx-table.py, with a pinned PDF hash and exact 8,493-row
census. It emits spec/h8sx-rows.jsonl with source tokens, lengths, masks,
values and variable field positions. All 16-bit opcode cells validate.
The source's missing page-673 AND operand label was visually confirmed;
its inferred two-memory indexed-long form assembles to exact printed bytes
for zero and all-one completions. The reusable probe records those two checks.

scripts/probe-h8sx-table.py independently checks 16,986 row completions
against GNU disassembly: 16,865 match length/mnemonic, with 121 review cases
retained in spec/h8sx-row-probe.json. docs/H8SX-TABLE.md explains constraints,
source anomalies and limits. No runtime acceptance changed in this increment.

Next: audit field constraints and the remaining raw-row discrepancies,
especially page 642 ADD.B @(d:32,Rs.W),@ERd+; compile the checked row facts
into opcode-group runtime rules and shared semantic operand mappings. Avoid
treating table-mask matches as sufficient validity: #xx:3 reserved zero
values, branch alignment, register groups and opcode aliases need explicit
handling. The larger goal remains active.

H8SX boundary grammar continuation (2026-09-26): the full extracted table is
now integrated into isa::insn_len through src/isa/sx_table.rs and generated
sx_table_data.rs. The old partial sx_length.rs was replaced. Generation
indexes 99,079 candidates by first word; it applies explicit nonzero short
immediate/displacement restrictions, serial register bounds, and relative
branch alignment. All 8,493 rows pass constrained minimum/maximum witnesses
and every truncation. Ordinary tests now reflect newly implemented MOVA,
EXTU/EXTS #2, immediate-to-memory MOV, and memory/indexed families.

The page-642 ADD.B @(d:32,Rs.W),@ERd+ row has two printed bit omissions,
confirmed visually and against neighboring .B/.L index variants. The source
value is preserved and a reviewed value is recorded separately. Both reviewed
field extremes independently assemble exactly. All 121 raw-row differences
are now classified and pinned: 78 GNU suffix spellings, eight stack aliases,
17 reserved-zero short immediates, 16 odd branches, two printed-bit samples.
The row probe fails on changed classification counts. It and generated-data
reproducibility are mandatory in scripts/conformance.sh.

The audit also corrected zero five-bit SHLL/SHLR counts (explicitly valid in
§2.2.99 [8]). Newly recognized BRA/BC, BRA/BS, BSR/BC and BSR/BS are guarded:
relocation returns DynamicBranch and reachability returns UnresolvedBranch,
so their relative targets are not silently copied. Further relocation support
is still part of the full goal.

Pinned H8SX zero-suffix census:
[7859 rejected, 56080 two-byte, 1392 four-byte, 204 six-byte, 1 eight-byte,
0 ten-byte, 0 twelve-byte, 0 fourteen-byte].
GNU first-word agreement is 57,677; both reject 7,357; GNU-only 502, no
crate-only acceptance or accepted-length disagreement. These oracle counts
are now pinned as well.

Verification: ordinary tests, Clippy -D warnings, Rust 1.58 all-target suite,
rustdoc -D warnings, package verification, mandatory GNU conformance including
row probes, and release operand audits all pass. Source coverage is 100%
across 4,425 regions, 2,623 lines, 122 functions. Latest logs are
/tmp/h8sx-table-{coverage-final,oracle-final,msrv,field-audit,doc,package}.log.

Next major task is full H8SX shared semantics and encoding/rendering.
The row JSON retains syntax and variable field offsets, suitable for compiling
typed operand descriptors. There are about 150 operand spellings, mostly
combinations of the existing Ea vocabulary. MOVA has nested indexed-memory
sources and will need a Copy-compatible representation rather than losing
semantic fields or storing raw bytes. Short d:2 is scaled by operand size;
byte indexes mean RnL, not RnH; multi-register endpoints and BRA/S flag bits
must retain semantics. Keep the existing subset APIs. Complete independent
assemble-back for the shared SX codec, full four-byte sweeps, wider operand
audits, divergence review, mutation testing and release requirements afterward.
The active six-phase goal is not complete.

Shared H8SX semantic continuation (2026-09-26): all 8,493 reviewed rows now
compile typed operand descriptors in scripts/h8sx_operands.py. Shared
semantics, verified encoding and rendering support all five targets. MOVA
retains nested memory indexing and distinguishes full four-bit RnH/En
sources from compact RnL/Rn forms. Encoding::SxAlternative retains a row
identity for noncanonical forms, without caching raw instruction bytes.

Five constrained samples per row include three asymmetric bit patterns.
All 42,465 samples round-trip. Independent GNU checks report 56,080 exact
two-byte results and, after row-sample deduplication, 42,315 exact, 12
semantically checked alternative forms, and 18 disassembly-only cases.
The latter are six manual-valid zero-count shifts and twelve GNU MOVA
shortening bugs; docs/CONFORMANCE.md records exact predicates and citations.
The oracle is mandatory and its counts are pinned.

The audit also guarded MOVSD.B's relative branch from silent relocation or
fallthrough analysis, and corrected ignored PC bit zero in direct/vector
control-flow resolution and internal-target refusal. Explicit regressions
cover these behaviors. The normal suite, Rust 1.58, Clippy, rustdoc and
mandatory conformance pass. Source coverage is 100% across 4,835 regions,
2,834 lines and 143 functions. Source coverage is not project completion.

Full four-byte sweeps are now running locally through examples/exhaustive.rs,
one target per process, three workers each. Logs /tmp/h8-exhaustive-{0..4}.log.
The harness enumerates every four-byte pattern at an odd-word buffer offset,
checks length/shared decode agreement, round-trips every four-byte decode,
and checks two-byte suffix independence. No sampling or skipped invalid
prefixes. Full results and pinning are not yet recorded. Next: complete
these sweeps and wider field audits, reviewed reverse-census divergences,
mode audit, mutation tests, remaining relocation capabilities and release
setup. The six-phase goal remains active.

Four-byte and field-audit completion checkpoint (2026-09-26): all five full
2³²-pattern sweeps passed, 21,474,836,480 candidates total. Counts and method
are pinned in examples/exhaustive.rs and spec/H8-ISA.md; scripts/exhaustive.sh
is required in every QA OS/MSRV matrix cell. Longer H8SX field generation
adds 163,471 witnesses: 163,435 exact GNU assemblies, 18 alternatives and
18 MOVA disassembly-only cases. Inherited legacy longer fields add 280/1274/
1274 exact cases across H8/300H, H8S/2000 and H8S/2600. Normal-mode GNU runs
pass all supported two-byte words on all applicable extended cores and the
full SX five-witness row corpus. GNU lacks separate middle/maximum selectors;
that limitation is explicit. The 502 SX zero-suffix reverse differences are
now individually pinned in tests/data/sx_reverse.tsv with manual reasons.

Created public repository https://github.com/MattJackson/h8-asm and origin.
No source push or release yet. New security/governance/contribution/assurance/
encoding-stability documents describe actual behavior, not inherited claims.
REUSE lint passes (102/102 files at that checkpoint); full source coverage
remains 4835 regions / 2834 lines / 143 functions, all 100%.

An EC2 mutation audit is ACTIVE and must be collected and cleaned up:
instance i-00f220ab40dd32e45, us-east-1, AWS profile matthew-admin,
54.234.190.89, c7i.8xlarge, Ubuntu 24.04. Instance shutdown auto-terminates
after 180 minutes from boot (~17:52 UTC). Connection/lifecycle IDs and
private temporary SSH key are under /tmp/h8-mutation (do not commit keys).
Remote source /home/ubuntu/h8-asm; log /home/ubuntu/mutation.log;
results /home/ubuntu/h8-asm/mutants.out. Run has 1801 mutations; snapshot
hash and command are in ROADMAP.md. It currently runs in 16 workers.
The launching SSH timed out after detaching, but pgrep and live outcomes
confirm the actual audit continues. Do not launch a duplicate. Save results,
terminate the instance, then delete its temporary security group/key pair.

Some initial missed mutants exposed missing boundary assertions, now added
locally in tests/{analysis,detour,image,relocate_asm}. These tests are newer
than the remote audit snapshot; rerun missed mutations against the final
source/tests before recording final results. Modulo-to-plus survivors in
analysis::flow and relocate::layout appear algebraically equivalent because
the downstream displacement calculation reduces modulo the same PC width.
Other redundant validation guards need individual assessment. Do not report
a partial audit as complete.

Next: finish the mutation audit and targeted regression verification; finalize
local gates, commit/push dev then qa, configure GitHub protection and finish
release/account setup. No crates.io token or local cargo credential store is
present. Searched integrations using the plugin-management skill and suggested
TinyFish for authenticated browser setup; it is NOT confirmed connected.
Continue independent work; do not ask routine permissions or pause the goal.


Final audit/release-candidate checkpoint (2026-09-26):

The preceding ACTIVE EC2 note is historical: the full audit completed, results
were downloaded, and the instance, security group and key pair were deleted.
All original survivors were reassessed/rechecked; the changed analysis module
received a complete rerun. Combined results: 1808 mutations, 1698 caught,
67 surviving equivalents, five timeouts, 38 unviable. docs/MUTATION-AUDIT.md
and docs/mutation-audit.json preserve the review and exact source/test hashes.

Both hosted QA runs 36261711662 and 36261807894 passed the full six-cell
OS/MSRV matrix and exhaustive sweeps. GitHub dev/qa protections and the main-only
release environment are configured. Source coverage for the final vector fix
is 4840 regions / 2835 lines / 144 functions, all 100%; this is source coverage,
not completion of release/account work. Current changes still need final QA.

The maintainer requested version 0.5.0 and provided a local crates.io token;
it is saved in Cargo credentials and the GitHub release environment secret.
The release workflow supports token bootstrap and explicit OIDC selection.
No token value belongs in this repository or its logs. No publication is yet
claimed. Codecov's first OIDC upload and OpenSSF registration remain pending.
ROADMAP.md is the current remaining-work list; older checkpoints above are
historical evidence, not current state. The six-phase goal remains active.

Codecov follow-up: OIDC upload succeeded in run 36263344962; the public API
confirms activation and a complete 100% report for 0e0cb92. README badge added.
The publishing token lacks Trusted Publishing configuration permission (403),
so the protected release environment token path remains selected.
