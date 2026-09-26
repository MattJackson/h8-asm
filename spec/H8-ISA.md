# H8 instruction coverage

Sources: H8/300 ADE-602-025 Appendix A/B; H8/300H REJ09B0213
§2.4–2.5; H8S REJ09B0139 §2.4–2.5; H8SX REJ09B0102 §2.4.
This is an initial map, not a claim of complete ISA support.

## 1. Instruction boundaries

Words are big-endian. `isa::insn_len` accepts a complete byte slice and an
explicit target/mode pair. It returns `None` for truncation, an unsupported
pair, an unrecognized encoding, or an unimplemented H8SX instruction. It does not return
operands or prove that an instruction is safe to execute.

The first word does not always determine length or validity. For example,
`0100 6907` is four bytes, whereas `0100 7870 6b27 ffffffff` is ten bytes
(H8S Table 2.2, MOV.L). Memory bit operations also require a later opcode,
sometimes following an address extension. A census with a fixed suffix cannot
classify every prefix as allocated or undefined.

## 2. Initial opcode map

The ranges below refer to the high byte of the first word. Length recognition
lives in `src/isa/length.rs`. Every row still lacks semantic decode/encode.
“Later” names additions within a row, not a claim that every pattern is valid.

| High byte | Family | Introduced / later additions | Length recognition |
|---|---|---|---|
| 00 | NOP | 300 | implemented |
| 01 | SLEEP and extension prefixes | 300; 300H long/control; H8S register groups; 2600 MAC | implemented through H8S |
| 02–07 | Control-register transfers and immediate logic | 300; H8S EXR; 2600 MAC registers | implemented through H8S |
| 08–1f | Register arithmetic, shifts, rotates, logic | 300; 300H word/long; H8S shift by two | implemented through H8S |
| 20–3f | Byte absolute moves | 300 | implemented |
| 40–4f | Conditional branches, d:8 | 300 | implemented |
| 50–53 | Multiply/divide | 300; 300H word | implemented through H8S |
| 54–5f | Returns, calls, jumps, traps and d:16 branches | 300; 300H extensions | implemented through H8S |
| 60–67 | Register bit operations and word logic | 300; 300H word logic | implemented through H8S |
| 68–6f | Memory moves and absolute bit prefixes | 300; 300H extended addresses; H8S absolute bit operations | implemented through H8S |
| 70–77 | Immediate bit operations | 300 | implemented |
| 78 | Extended displacement prefix | 300H | implemented through H8S |
| 79–7a | Immediate word/long operations | 300 word MOV; 300H extensions | implemented through H8S |
| 7b | EEPMOV | 300 byte; 300H word | implemented through H8S |
| 7c–7f | Memory bit operations | 300 | implemented through H8S |
| 80–ff | Byte immediate operations | 300 | implemented |

H8SX extends these spaces. Only four operation-only rows have length
recognition: NOP `0000`, SLEEP `0180`, RTS `5470`, and RTE `5670` (H8SX
§2.4). All other H8SX encodings are currently refused. Its full §2.4 table
review and exact maximum-length derivation remain open; do not use the current
ten-byte test buffers as an H8SX maximum. The ADD.B
`@(d:32,ERs),@(d:32,ERd)` row in §2.4 contains two 32-bit displacements,
and is a concrete example longer than the H8S witnesses here.

## 3. Manual boundary witnesses

`tests/length.rs` checks 2/4/6/8/10-byte recognition, every truncation of the
longer witnesses, and trailing-byte independence. It includes EEPMOV's fixed
second word, separate read-only/read-modify-write bit opcodes, EXR, MAC, TAS's
ER0/1/4/5 restriction (H8S Table 2.2 note 3), and the H8/300H zero high byte
in a 24-bit absolute address extension versus H8S's 32-bit address.

These are selected manual witnesses, not independent conformance evidence.

## 4. Counts and method

The pinned regression census enumerates all 65,536 first words with eight
zero suffix bytes, in normal mode. Every recognized result is checked at its
exact length and every shorter slice; all four modes are checked against the
target support table. Counts describe this probe construction only.

| Target | Rejected | 2 bytes | 4 bytes | 6 bytes | 8 bytes | 10 bytes |
|---|---:|---:|---:|---:|---:|---:|
| H8/300 | 11,506 | 53,556 | 474 | 0 | 0 | 0 |
| H8/300H | 7,647 | 56,520 | 1,249 | 120 | 0 | 0 |
| H8S/2000 | 7,295 | 56,872 | 1,249 | 120 | 0 | 0 |
| H8S/2600 | 7,262 | 56,905 | 1,249 | 120 | 0 | 0 |
| H8SX | 65,532 | 4 | 0 | 0 | 0 | 0 |

Zero counts for eight and ten bytes reflect the fixed suffix, not absent
instructions. H8SX rejections reflect missing implementation, not undefined
instructions. Full suffix/operand sweeps, semantic round trips, binutils
conformance, and the phase-3 exhaustive audit have not been performed.
