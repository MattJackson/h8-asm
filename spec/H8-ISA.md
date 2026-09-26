# H8 instruction coverage

Sources: H8/300 ADE-602-025 Appendix A/B; H8/300H REJ09B0213
§2.4–2.5; H8S REJ09B0139 §2.4–2.5; H8SX REJ09B0102 §2.4.
The shared semantic codec covers recognized instructions on all five targets
with typed operands, exact-width encoding and Renesas rendering. The H8SX
implementation follows all 8,493 reviewed manual rows. The full four-byte
census and longer field audits are recorded below; release setup is tracked
in ../ROADMAP.md.

## 1. Instruction boundaries

Words are big-endian. `isa::insn_len` accepts a complete byte slice and an
explicit target/mode pair. It returns `None` for truncation, an unsupported
pair, or an unrecognized encoding. It does not return
operands or prove that an instruction is safe to execute.
For H8/300 through H8S, odd displacements in Bcc/BSR are rejected:
H8/300 §2 says the signed displacement must be even, and the later manuals
retain the even branch-destination requirement. H8SX separately allocates
BRA/S with bit 0 set, so it does not use this legacy rule. H8SX BSR d:8 and
the d:16 Bcc/BSR forms still require even destinations.

The first word does not always determine length or validity. For example,
`0100 6907` is four bytes, whereas `0100 7870 6b27 ffffffff` is ten bytes
(H8S Table 2.2, MOV.L). Memory bit operations also require a later opcode,
sometimes following an address extension. A census with a fixed suffix cannot
classify every prefix as allocated or undefined.

**The maximum H8SX instruction length is 14 bytes.** H8SX §2.4 Table 2.2
provides at most three 16-bit opcode words and at most two 32-bit operand
extensions, giving 6 + 8 = 14 bytes. The `ADD.B
@(d:32,ERs),@(d:32,ERd)` row attains the bound: its three opcode words and
two four-byte displacements total 14 bytes. The full table's mnemonic column
was checked for 32-bit fields: 589 rows contain two `:32` fields, and none
contains three. The earlier cores' instruction-code tables are shorter; H8S
§2.4 Table 2.2 ends at a tenth byte. `isa::MAX_INSN_LEN` pins the family bound.

## 2. Opcode maps

Legacy lengths are implemented in src/isa/length.rs and their shared
semantics in src/isa/legacy.rs. H8SX lengths now use src/isa/sx_table.rs
and generated sx_table_data.rs instead of the former partial recognizer.

The H8SX table covers every one of the 8,493 printed §2.4 rows. Generation
retains page provenance, fixed bits and field widths; explicit restrictions
handle nonzero short arithmetic immediates, scaled short displacements,
serial register groups and even relative destinations. The first-word index
contains 99,079 row candidates and avoids scanning the whole table per
instruction. Every row has constrained minimum/maximum-field witnesses and
all truncations checked. This proves coverage of the extracted row grammar,
separate from the full four-byte sweep recorded below.

The grammar includes memory-to-memory arithmetic, all table EA extensions,
MOV/MOVA, shifts, bit operations, control flow, register groups, multiply and
divide forms, and control-register operations. See the reproducible
[row map, source anomalies and oracle results](../docs/H8SX-TABLE.md).
The shared semantic codec covers H8SX through generated typed operand
descriptors, including nested MOVA operands. The retained separate
H8SX subset API is described in §6.

BRA/BC, BRA/BS, BSR/BC, BSR/BS and MOVSD.B are explicitly refused by
relocation and conservative reachability, preventing stale relative targets
from being copied as position-independent instructions.

## 3. Manual boundary witnesses

`tests/length.rs` checks 2/4/6/8/10/12/14-byte recognition, every truncation of the
longer witnesses, and trailing-byte independence. It includes EEPMOV's fixed
second word, separate read-only/read-modify-write bit opcodes, EXR, MAC, TAS's
ER0/1/4/5 restriction (H8S Table 2.2 note 3), and the H8/300H zero high byte
in a 24-bit absolute address extension versus H8S's 32-bit address.
For H8SX, it checks all sixteen Bcc conditions and reserved low bits, the
BRA/S delay-slot opcode, all register pairs in the 14-byte ADD.B row, and the
destination EA length variants. The 32,768 byte-immediate words and 8,704
register/absolute-move words are checked exhaustively.

These are selected manual witnesses, not independent conformance evidence.

## 4. Counts and method

The pinned regression census enumerates all 65,536 first words with twelve
zero suffix bytes, in normal mode. Every recognized result is checked at its
exact length and every shorter slice; all four modes are checked against the
target support table. Counts describe this probe construction only.

| Target | Rejected | 2 bytes | 4 bytes | 6 bytes | 8 bytes | 10 bytes | 12 bytes | 14 bytes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| H8/300 | 13,682 | 51,380 | 474 | 0 | 0 | 0 | 0 | 0 |
| H8/300H | 9,823 | 54,344 | 1,249 | 120 | 0 | 0 | 0 | 0 |
| H8S/2000 | 9,471 | 54,696 | 1,249 | 120 | 0 | 0 | 0 | 0 |
| H8S/2600 | 9,438 | 54,729 | 1,249 | 120 | 0 | 0 | 0 | 0 |
| H8SX | 7,859 | 56,080 | 1,392 | 204 | 1 | 0 | 0 | 0 |

Zero counts for ten bytes and longer reflect the fixed suffix, not absent
instructions. Rejection in this probe does not classify every completion of
a first word as undefined. The full four-byte sweep and longer operand-field
audits are recorded below, separately from this fixed-suffix probe.

The opt-in `tests/binutils_legacy.rs` probe uses the same zero-suffix method
for the four older targets. After enforcing the manual's even d:8 branch
displacement rule, binutils 2.47 has no accepted-length disagreements or
crate-only decodes in that probe. Its matching recognized counts are 51,854
(H8/300), 55,713 (H8/300H), 56,065 (H8S/2000), and 56,098 (H8S/2600).
Binutils also decodes 6,325, 2,466, 2,114, and 2,081 additional first words
respectively; some are later-core instructions decoded in older-core mode.
Those counts cannot establish that the extra patterns belong to each target.

## 5. Independent first-word probe

`tests/binutils_first_word.rs` is an ignored, opt-in comparison against GNU
binutils. It assembles all 65,536 H8SX first words with fourteen zero padding
bytes per candidate, disassembles with `objdump -d -z -w`, and compares the
length at each 16-byte slot with `isa::insn_len` on the same zero suffix.
With binutils 2.47, 57,677 slots agree on a recognized length, 7,357 are
rejected by both, and 502 are decoded only by binutils. No slot is accepted
only by this crate, and no accepted slot has a length disagreement. This is
an independent boundary check for one suffix, not full conformance.

Some binutils-only slots are aliases the manual does not allocate. For
example, binutils decodes `57 40` as `TRAPA #0`, although the H8SX §2.4
TRAPA row fixes bit 6 to zero. It also accepts `0b 08` as `ADDS #1,ER0`
despite the fixed-zero bit before the three-bit ER field. Those patterns
remain rejected here. Every H8SX GNU-only triple now has a reviewed manual
reason in tests/data/sx_reverse.tsv; its count is not a missing-instruction count.

`tests/binutils_assemble_back.rs` provides a stronger check for the semantic
H8/300 subset: it enumerates all 2,692 two-byte words the decoder accepts,
verifies the crate's encoder produces each original word, renders Renesas
assembly, translates only the Renesas `$` location counter to GNU gas's `.`,
assembles, links to resolve PC-relative relocations, and compares the final
`.text` bytes with the originals. All 2,692 matched using binutils 2.47.
The linker step matters: comparing unresolved object bytes would give a
false result for branches. This does not cover the other semantic or
length-only families.

`tests/binutils_sx_assemble_back.rs` repeats that exact-byte check for the
current H8SX semantic subset, translating Renesas `$` and `H'` to GNU gas
`.` and `0x`. Using binutils 2.47 with the `h8300sxelf` linker emulation,
all 39,297 supported two-byte words, 140 representative four-byte forms,
and 1,064 word/full-width long-immediate boundary cases match. The four-byte cases include every Bcc condition, signed branch
boundaries, absolute-address extremes, and every byte-register code for the
supported absolute move form. This does not establish conformance for other
H8SX instructions or for all four-byte operand combinations.


## 6. Retained narrow H8SX API (2026-09-26)

All rows below cite REJ09B0102 §2.4 Table 2.2. Registers, immediate bits,
and exact instruction lengths are retained by decode and encode.

| Form | Operations | Encoding / field limits |
|---|---|---|
| Byte immediate | ADD, ADDX, CMP, SUBX, OR, XOR, AND, MOV | `8r`–`fr`, 8-bit literal, 16 byte registers |
| Word immediate | MOV, ADD, CMP, SUB, OR, XOR, AND | `79 0r`–`79 6r`, 16-bit literal, 16 word registers |
| Full-width long immediate | MOV, ADD, CMP, SUB, OR, XOR, AND | `7a 0r`–`7a 6r`, bit 3 clear, 32-bit literal, 8 ER registers |
| Byte register pair | ADD, MOV, ADDX, OR, XOR, AND, SUB, CMP, SUBX | `08/0c/0e/14/15/16/18/1c/1e`, two four-bit register fields |
| Word register pair | ADD, MOV, SUB, CMP, OR, XOR, AND | `09/0d/19/1d/64/65/66`, two four-bit register fields |
| Long register pair | ADD, MOV, SUB, CMP | `0a/0f/1a/1f`, low-byte bit 7 set, bit 3 clear, two three-bit fields |

This adds 24,576 byte-immediate words and 4,352 register-pair words to the
previous 10,369-word semantic subset, giving 39,297. The census enumerates
all 65,536 two-byte patterns and checks every accepted result against length
recognition and the encoder. The external assembler checks the rendered text
of every accepted word as well.

Compact long immediates, 16-bit immediates to long registers, prefixed long
logical operations, word/long ADDX/SUBX, and other memory forms remain
outside this narrow API; they are supported by the shared codec. They must not be inferred from a shared opcode
byte. The tests explicitly refuse unsupported operation/size combinations
and out-of-range register fields. See
[`../docs/CONFORMANCE.md`](../docs/CONFORMANCE.md) for reproduction.


`tests/sx_operand_audit.rs` exhaustively checks the implemented word-immediate
operand space: 7 operations × 16 registers × 65,536 literals = **7,340,032**
four-byte patterns. Each agrees with the expected typed operation, length,
and verified encoder. It also checks 17 branch/call forms × 65,536 raw
16-bit displacements × 4 modes = **4,456,448** cases, accepting every even
displacement and refusing every odd displacement. Both release-mode audits
passed locally on 2026-09-26 and are required by the QA test matrix. These
are internal consistency audits of the named families; the independent
assembler still samples longer forms. The complete 2³²-pattern audit for
every target is now recorded below.


## Shared legacy semantics

The `isa::{Insn, Operand, Reg, Ea, Size, Encoding}` vocabulary is consumed by
`decode_insn`, `encode_insn`, and `disassemble_insn`. The original narrow
H8/300 and H8SX APIs remain available. The shared API covers H8SX as well.

Legacy semantic coverage includes CCR/EXR and MAC transfers, immediate and
register operations, memory moves, bit operations, shifts/rotates, signed and
unsigned multiply/divide, branches/calls/traps, register groups, TAS, and
EEPMOV. Operand widths and the allowed long-displacement MOV.L store alias
are retained for byte-exact encoding. Values that would be truncated or
reinterpret fields are refused.

The legacy first-word zero-suffix census is pinned, every recognized
two-byte word assembles independently, and selected extended fields are
enumerated. See [conformance evidence](../docs/CONFORMANCE.md) for counts and
explicit distinctions between exact matches, aliases and GNU divergences.
H8S LDM/STM register groups follow §2.2.36/§2.2.63 rather than H8SX's less
restrictive serial-register rule. Recognition of ER7-containing groups from
the generic software manual is not permission to use them on products whose
hardware manual prohibits them.


## Shared H8SX semantic validation

All 56,080 supported two-byte encodings round-trip through the typed codec
and independently assemble exactly. All 8,493 manual rows have five valid
witnesses, including asymmetric field values. Their 42,465 round trips
preserve register classes, both EA extension fields, scaled displacements,
register-list endpoints and noncanonical forms. The independent row oracle
reports 42,315 exact cases, 12 alternative forms and 18 disassembly-only GNU
limitations after deduplication; see docs/CONFORMANCE.md for the distinctions.
Adversarial operand edits check that successful encoding never silently
truncates or changes any semantic field. This remains separate from the
full 2³²-pattern audit.


## Full four-byte census (2026-09-26)

scripts/exhaustive.sh enumerates all 2³² four-byte patterns for each target.
Each candidate starts at byte offset two in a guarded buffer (an odd-word
offset), with exactly four bytes exposed. Shared semantic acceptance and
length recognition must agree. Every four-byte instruction encodes back to
its original bytes. Every two-byte result must equal the independently
decoded prefix, verifying suffix independence; that prefix also round-trips.
Unsupported/truncated candidates are counted rather than skipped. These
are pattern counts, so a two-byte instruction appears once per 65,536 suffixes.

| Target / mode | Rejected or truncated | Two-byte prefix | Four-byte instruction |
|---|---:|---:|---:|
| H8/300 normal | 896,617,087 | 3,367,239,680 | 31,110,529 |
| H8/300H advanced | 652,133,790 | 3,561,488,384 | 81,345,122 |
| H8S/2000 advanced | 629,064,042 | 3,584,557,056 | 81,346,198 |
| H8S/2600 advanced | 626,901,290 | 3,586,719,744 | 81,346,262 |
| H8SX maximum | 526,073,994 | 3,675,258,880 | 93,634,422 |

Every row totals 4,294,967,296; all five runs passed. The census is pinned in
examples/exhaustive.rs and the full sweep is mandatory in the QA OS/MSRV
matrix. The other modes and longer forms remain distinct checks; these
figures do not measure hardware behavior or independent GNU conformance.

For longer H8SX instructions, scripts/compile-h8sx-table.py additionally
generates 163,471 field witnesses. Every register/small field takes every
value independently, with reserved values constrained as in the grammar;
wide extensions take zero, one, two, signed midpoint boundaries and unsigned
maximum boundaries. Every witness round-trips, and independent GNU results
are recorded in docs/CONFORMANCE.md. This field audit is not enumeration of
all possible combinations of large extensions.
