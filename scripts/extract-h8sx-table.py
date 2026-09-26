#!/usr/bin/env python3
"""Extract the H8SX Rev. 4 §2.4 encoding facts using PDF cell coordinates.

Requires Poppler pdftotext. This is a development tool, not a library build
dependency. The PDF checksum and row census fail closed on source/layout
changes. Output records preserve printed tokens and source page provenance;
they are evidence for a row map, not by themselves a valid-instruction grammar.
"""
import xml.etree.ElementTree as E
import re, collections, json
import argparse
import hashlib
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--pdf", type=Path, default=Path("spec/H8SX-software-manual.pdf"))
parser.add_argument("--output", type=Path, default=Path("spec/h8sx-rows.jsonl"))
args = parser.parse_args()
digest = hashlib.sha256(args.pdf.read_bytes()).hexdigest()
assert digest == "95c7468b55e4a5be28a29566c6cc384cfe118f51deb5f2cef99e53e37cb22055", "Unexpected manual revision"
with tempfile.TemporaryDirectory(prefix="h8sx-table-") as work:
    xml = Path(work) / "table.xml"
    subprocess.run(["pdftotext", "-f", "653", "-l", "907", "-bbox-layout",
                    str(args.pdf), str(xml)], check=True)
    document = xml.read_bytes()

ns='{http://www.w3.org/1999/xhtml}'
root=E.fromstring(document)
rows=[]; failures=[]; widths=collections.Counter()
for page,p in enumerate(root.iter(ns+'page'),635):
    words=[(float(w.get('xMin')),float(w.get('yMin')),float(w.get('xMax')),w.text or '') for w in p.iter(ns+'word') if 0<float(w.get('yMax'))-float(w.get('yMin'))<4.5]

    headers=sorted([w for w in words if w[3]=='15'],key=lambda w:w[0])
    hy=headers[0][1]
    head=sorted([w for w in words if abs(w[1]-hy)<.1 and w[0]>300])
    eights=[w for w in head if w[3]=='8']
    assert len(eights)==3,(page,eights)
    groups=[]
    for h in eights:
        z=next(w for w in head if w[0]>h[0] and w[3]=='0')
        eight=(h[0]+h[2])/2; last=(z[0]+z[2])/2
        preceding=[w for w in head if w[3]=='15' and 0<h[0]-w[0]<80]
        if preceding:
            first=(preceding[-1][0]+preceding[-1][2])/2
            step=(last-first)/15
        else:
            step=(last-eight)/8
            first=last-15*step
        groups.append((first,last,step))
    # Mnemonic column starts near x245, while x219 is the family heading.
    starts=[w for w in words if 238<w[0]<252 and w[1]>hy and re.fullmatch(r'[A-Z][A-Z0-9/]*(?:\.[BWL])?',w[3])]
    starts.sort(key=lambda w:w[1])
    for ri,start in enumerate(starts):
        end=starts[ri+1][1]-.8 if ri+1<len(starts) else start[1]+9
        line=sorted([w for w in words if start[1]-.8<=w[1]<end],key=lambda w:w[0])
        syntax_words=sorted([w for w in line if 238<w[0]<groups[0][0]-4],key=lambda w:(round(w[1]),w[0]))
        operands = ''.join(w[3] for w in syntax_words if w != start)
        syntax = start[3] + (' ' + operands if operands else '')
        sections=[]; error=[]
        for gi,(first,last,step) in enumerate(groups):
            bits=0; tokens=[]
            for w in line:
                c=(w[0]+w[2])/2
                if c<first-step*.6 or c>last+step*.6: continue
                width=round(2*(c-first)/step)-2*bits+1
                widths[(w[3],width)]+=1
                if not 1<=width<=16: error.append((gi,bits,w[3],round(c,3),width))
                tokens.append((w[3],width)); bits+=width
            if tokens and bits!=16: error.append(('total',gi,bits))
            if tokens: sections.append(('opcode',tokens))
            # Extension columns occur after word 2 and word 3.
            if gi>=1 or len(groups)==1:
                end=groups[gi+1][0]-step*.6 if gi+1<len(groups) else 750
                ext=[w[3] for w in line if last+step*.6<(w[0]+w[2])/2<end]
                if ext: sections.append(('extension',ext))
        if error: failures.append((page,syntax,error,sections))
        rows.append(dict(page=page,syntax=syntax,sections=sections))

assert not failures, failures
assert len(rows) == 8493, len(rows)
lengths = collections.Counter()
for row in rows:
    length = 0
    for kind, fields in row["sections"]:
        if kind == "opcode":
            assert sum(width for _, width in fields) == 16, row
            for token, width in fields:
                assert token in {"0", "00", "1", "2", "3", "4", "5", "6", "7",
                                 "8", "9", "A", "B", "C", "D", "E", "F",
                                 "rs", "rd", "rn", "rm", "r", "rn+1", "rn+2",
                                 "rn+3", "x", "d", "a", "v"}, row
                assert width in (1, 2, 3, 4, 5, 8, 16), row
            length += 2
        else:
            assert all(token in {"d", "a", "x"} for token in fields), row
            length += 2 * len(fields)
    assert 2 <= length <= 14 and length % 2 == 0, row
    mask = value = 0
    fields = []
    bit_offset = 0
    for kind, tokens in row["sections"]:
        tokens = tokens if kind == "opcode" else [(token, 16) for token in tokens]
        for token, width in tokens:
            mask <<= width
            value <<= width
            if re.fullmatch(r"[0-9A-F]+", token):
                literal = int(token, 16)
                assert literal < (1 << width), (row, token, width)
                mask |= (1 << width) - 1
                value |= literal
            else:
                fields.append([token, bit_offset, width])
            bit_offset += width
    assert bit_offset == length * 8
    row["mask"] = mask.to_bytes(length, "big").hex()
    row["value"] = value.to_bytes(length, "big").hex()
    row["fields"] = fields
    row["length"] = length
    lengths[length] += 1
    if row["page"] == 673 and row["syntax"] == "AND.B":
        # The omission is present in the rendered PDF, not just its text layer.
        # Keep the source blank and label the inferred syntax separately.
        row["inferred_syntax"] = "AND.B @(d:32,ERs.L),@(d:32,ERd.L)"
        row["note"] = "Operand text absent in rendered source; inference is independently assembled by probe-h8sx-table.py."
    if row["page"] == 642 and row["syntax"] == "ADD.B @(d:32,Rs.W),@ERd+":
        assert row["value"] == "78066a24000000000010"
        row["reviewed_value"] = "78066a2c000000008010"
        row["note"] = ("Printed row omits bit 3 of byte 4 and bit 7 of byte 9. "
                       "Adjacent source-index variants on pp.642-643 retain both bits. "
                       "The reviewed encoding is independently assembled by the probe.")
    if row["page"] == 651 and any(
        token == "00" for kind, fields in row["sections"] if kind == "opcode"
        for token, _ in fields
    ):
        row["note"] = "Overlapping 00 glyph occupies one zero-bit cell."
assert lengths == {2: 223, 4: 734, 6: 1805, 8: 2237, 10: 2124, 12: 980, 14: 390}, lengths
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows))
print(f"{len(rows)} rows from manual pages 635–889; lengths {dict(sorted(lengths.items()))}")
