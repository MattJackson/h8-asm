# Local Renesas specs (reference only — not shipped in the crate)

These are the primary sources for every encoding claim in `src/`. Each PDF has a
sibling `.txt` file: a pypdf text dump with one `===== PDFPAGE n =====` marker
per page, so the encodings can be grepped:

```sh
grep -n "2.4 List of Instruction Codes" spec/H8SX.txt
awk '/PDFPAGE 653 /,/PDFPAGE 655 /' spec/H8SX.txt
```

The dumps flatten tables badly. The instruction-code tables are where that
hurts most, because a column that loses its alignment turns into a bit field
read at the wrong position. When you need to read a table rather than search
for it, re-extract just those pages with the layout preserved:

```sh
pdftotext -layout -f 653 -l 660 spec/H8SX-software-manual.pdf - | less
```

The PDFs are RC4 "encrypted" with an empty user password, which only stops
them being edited. pypdf and poppler both read them without a password.

| File | Doc | Pages | Why it's here |
|---|---|---|---|
| `H8-300-programming-manual.pdf` | H8/300 Programming Manual, ADE-602-025 Rev.1.0 | 141 | The original 16-bit core. 57 instructions, all 2 or 4 bytes long, 8 addressing modes. **Appendix A** is the operation code map and **Appendix B** the instruction set list. It is the base that every later encoding table extends. |
| `H8-300H-software-manual.pdf` | H8/300H Series Software Manual, REJ09B0213-0300 Rev.3.00 | 277 | Adds 32-bit ER registers, the 24-bit address space (normal/advanced modes) and the `01xx`-prefixed long forms. **§2.4** is the instruction codes and **§2.5** the operation code map. |
| `H8S-2600-2000-software-manual.pdf` | H8S/2600, H8S/2000 Series Software Manual, REJ09B0139-0400 Rev.4.00 | 341 | Adds EXR, `LDM`/`STM`, `TAS`, and the `MAC` family (2600 only). Check the full delta against §1 before relying on this list. **§2.4** (Table 2.2) is the instruction codes and **§2.5** (Table 2.3) the operation code map. |
| `H8SX-software-manual.pdf` | H8SX Family Software Manual, REJ09B0102-0400 Rev.4.00 | 936 | The superset. It has four CPU modes (normal/middle/advanced/maximum), 11 addressing modes including PC-relative and index-register forms, `MOVA`, `BRA/S` (delay slot), `RTS/L`, bit-field and block-transfer instructions, and VBR/SBR. **§1.7.3** covers formats, **§1.8** addressing, **§2.3** the instruction set list (PDF p.291) and **§2.4** the list of instruction codes (manual p.635 = PDF p.653; manual page + 18 = PDF page). |
| `H8-compiler-assembler-v7-manual.pdf` | H8S, H8/300 C/C++ Compiler, Assembler, Optimizing Linkage Editor User's Manual, REJ10J2039-0100 Ver.7.00 | 1174 | Renesas' own assembler, and so the canonical **mnemonic syntax**, the role UAL plays for Arm. It also documents the `cpu=` names for every core and mode (grep `H8SX maximum mode`) and "Executable Instructions" (§11.2). |

## Provenance

All five were downloaded from renesas.com on 2026-09-25 at the URLs listed in
`LICENSES/LicenseRef-Renesas-Documentation.txt`. They are Renesas' copyright and
are excluded from the published crate.

## Other sources, used as oracles rather than as specs

- **GNU binutils, `h8300` target** (`opcodes/h8300-dis.c`,
  `include/opcode/h8300.h`, and `gas` with `.h8300h` / `.h8300s` /
  `.h8300sx` and the `n` normal-mode suffixes). This is the independent
  implementation used in the conformance harness. Treat it as corroboration,
  never as a citation: where it and the manual disagree, the manual wins and
  the disagreement is written down.
- **MAME `src/devices/cpu/h8/`**. Its H8/H8S disassembler tables are a possible
  second oracle. Check how far its H8SX coverage goes before relying on it.
