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
  README badges, then do the first `0.1.0` release via dev → qa → main.

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

Planning estimate, made at this checkpoint: 1–3 weeks of focused work for
reliable length recognition across all five targets, and 8–16 weeks for the
full six-phase scope. These are rough effort estimates, not release dates.
H8SX table complexity and independent conformance findings are the largest
unknowns.
