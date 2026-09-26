# H8SX encoding-table intake

scripts/extract-h8sx-table.py extracts encoding facts from the locally
stored H8SX Rev. 4 software manual, §2.4, printed pages 635–889 (PDF pages
653–907). It requires Python 3 and Poppler's pdftotext; neither is a
library build dependency. The PDF SHA-256 is pinned.

    python3 scripts/extract-h8sx-table.py
    python3 scripts/probe-h8sx-table.py

The output is spec/h8sx-rows.jsonl, one record per printed mnemonic row.
Each record retains the page, source syntax, opcode tokens and widths, EA
extension tokens, instruction length, fixed-bit mask/value, and variable
field offsets measured from the first instruction bit. These are encoding
facts extracted from the manual, not copied explanatory text. The generated
files remain under the existing unpublished spec/ tree.

The extractor checks all opcode words total 16 bits, all literals fit their
cells, all extension tokens are known, and every resulting instruction is
between two and fourteen bytes. Its pinned census is:

| Bytes | Printed rows |
|---:|---:|
| 2 | 223 |
| 4 | 734 |
| 6 | 1,805 |
| 8 | 2,237 |
| 10 | 2,124 |
| 12 | 980 |
| 14 | 390 |
| Total | 8,493 |

This is a row count, not a count of distinct legal encodings. Rows include
aliases and variable fields with additional constraints. For example,
register-group endpoints must not wrap, some immediate values are excluded,
and relative branch destinations must be aligned. The runtime generator applies the explicit restrictions for nonzero short
immediates and displacements, nonwrapping register groups, and branch alignment.

The extractor uses cell coordinates because whitespace extraction loses
register widths and the distinction between opcode bits and 16-bit EA
extension words. It handles wrapped syntax and excludes larger-font prose
following the table. Known source anomalies are retained explicitly:

- Page 651 has an overlapping 00 glyph in a single zero-bit cell.
- Page 696's third 15 header loses its 5 in text extraction; its 8 and
  0 headers define the same bit grid.
- Page 673's first AND.B row has no operand label, including in the rendered
  original PDF. The source syntax remains AND.B, with a separately recorded
  inference @(d:32,ERs.L),@(d:32,ERd.L). The probe independently assembles
  its zero and all-one field completions to the exact printed opcode bytes.

The raw GNU probe fills variable bits with zeros and ones and places each
sample at the start of its own sixteen-byte slot. With the pinned local
binutils build, 16,865 of 16,986 samples have matching lengths and mnemonics.
The remaining 121 are preserved in spec/h8sx-row-probe.json. Many are
spelling differences or intentionally invalid field completions; they have
not been silently counted as matches. The report also records the two
exact-byte AND checks separately.

The 121 raw differences are now classified: 78 GNU size-suffix spellings,
eight stack-move aliases, 17 reserved zero short-immediate completions,
16 odd branch displacements, and two samples of one printed opcode
inconsistency. These counts are pinned; an unexpected difference fails.

Page 642's ADD.B @(d:32,Rs.W),@ERd+ omits bit 3 in byte 4 and bit 7 in byte 9.
Both omissions are visible in the rendered PDF. Adjacent source-index
variants on pages 642–643 retain those bits. The printed value is preserved,
with a separate reviewed value; both zero/all-one reviewed completions
assemble exactly to GNU's encoding. This is a documented source correction,
not a silent replacement of manual facts with GNU output.

The runtime generator is scripts/compile-h8sx-table.py. It emits the matcher
data and first-word candidate index; --check proves the checked-in data
matches the generator. The conformance script now requires this check and
the full row probe. The row matcher has replaced the partial H8SX length
recognizer. Each row is tested at both constrained field extremes and every
truncation. Zero is valid for five-bit SHLL/SHLR counts (§2.2.99 [8]); the
previous recognizer's erroneous refusal was removed.

The generator also compiles typed operand descriptors through
scripts/h8sx_operands.py. Shared decoding, verified encoding and rendering
cover every row, retaining operand widths and noncanonical form identities.
The 42,465 row witnesses include both constrained extremes and three
asymmetric bit patterns; asymmetry checks independent source/destination
fields, including nested MOVA displacements. These pass typed round trips.
The independent semantic oracle is documented in CONFORMANCE.md. Full
four-byte and wider operand-field sweeps have passed and are documented
separately in spec/H8-ISA.md and CONFORMANCE.md.
