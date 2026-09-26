#!/usr/bin/env python3
"""Compare raw manual-row completions with GNU disassembly.

This development probe does not establish operand validity or assemble-back
conformance. Every row is completed with zero and one variable bits, then
placed in an independent 16-byte slot. Mismatches are retained for review.
"""
import argparse
import collections
import json
from pathlib import Path
import re
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--rows", type=Path, default=Path("spec/h8sx-rows.jsonl"))
parser.add_argument("--tools", type=Path, default=Path("target/h8-binutils/install/bin"))
parser.add_argument("--output", type=Path, default=Path("spec/h8sx-row-probe.json"))
args = parser.parse_args()
tools = args.tools.resolve()
rows = [json.loads(line) for line in args.rows.read_text().splitlines()]
cases = []
source = [".h8300sx", ".text"]
for index, row in enumerate(rows):
    mask = bytes.fromhex(row["mask"])
    value = bytes.fromhex(row["value"])
    for fill in (0, 255):
        encoded = bytes(v | (fill & ~m) for m, v in zip(mask, value))
        cases.append((index, fill, row, encoded))
        source.append(".byte " + ",".join(map(str, encoded)))
        source.append(f".space {16-len(encoded)},0")

with tempfile.TemporaryDirectory(prefix="h8sx-probe-") as work:
    work = Path(work)
    (work / "rows.s").write_text("\n".join(source) + "\n")
    result = subprocess.run([str(tools / "h8300-elf-as"), "-o", "rows.o", "rows.s"],
                            cwd=work, capture_output=True, check=True)
    assert not result.stderr, result.stderr
    result = subprocess.run([str(tools / "h8300-elf-objdump"), "-d", "-z", "-w", "rows.o"],
                            cwd=work, capture_output=True, check=True)
    assert not result.stderr, result.stderr
    listing = result.stdout.decode()
    inferred = next(row for row in rows if row.get("inferred_syntax"))
    inferred_checks = 0
    corrected_checks = 0
    for register, fill in [(0, 0), (7, 255)]:
        displacement = 0xffffffff if fill else 0
        source = (".h8300sx\n.text\n"
                  f"and.b @(0x{displacement:x}:32,er{register}.l),"
                  f"@(0x{displacement:x}:32,er{register}.l)\n")
        (work / "inferred.s").write_text(source)
        for executable, arguments in [
            ("as", ["-o", "inferred.o", "inferred.s"]),
            ("objcopy", ["-O", "binary", "-j", ".text", "inferred.o", "inferred.bin"]),
        ]:
            result = subprocess.run([str(tools / ("h8300-elf-" + executable)), *arguments],
                                    cwd=work, capture_output=True, check=True)
            assert not result.stderr, result.stderr
        expected = bytes(v | (fill & ~m) for m, v in
                         zip(bytes.fromhex(inferred["mask"]), bytes.fromhex(inferred["value"])))
        assert (work / "inferred.bin").read_bytes() == expected
        inferred_checks += 1

    corrected = next(row for row in rows if row.get("reviewed_value"))
    for register, fill in [(0, 0), (7, 255)]:
        displacement = 0xffffffff if fill else 0
        source = (".h8300sx\n.text\n"
                  f"add.b @(0x{displacement:x}:32,r{register}.w),@er{register}+\n")
        (work / "corrected.s").write_text(source)
        for executable, arguments in [
            ("as", ["-o", "corrected.o", "corrected.s"]),
            ("objcopy", ["-O", "binary", "-j", ".text", "corrected.o", "corrected.bin"]),
        ]:
            result = subprocess.run([str(tools / ("h8300-elf-" + executable)), *arguments],
                                    cwd=work, capture_output=True, check=True)
            assert not result.stderr, result.stderr
        expected = bytes(v | (fill & ~m) for m, v in
                         zip(bytes.fromhex(corrected["mask"]), bytes.fromhex(corrected["reviewed_value"])))
        assert (work / "corrected.bin").read_bytes() == expected
        corrected_checks += 1

counts = collections.Counter()
mismatches = []
classifications = collections.Counter()
seen = set()
for line in listing.splitlines():
    match = re.match(r"\s*([0-9a-f]+):\s*\t([^\t]+)\t(.*)", line)
    if not match:
        continue
    offset = int(match[1], 16)
    if offset % 16:
        continue
    index, fill, row, encoded = cases[offset // 16]
    seen.add(offset // 16)
    actual_length = len(match[2].split())
    actual = match[3].strip()
    expected_name = row["syntax"].split()[0].lower()
    actual_name = actual.split()[0]
    if actual_length == len(encoded) and actual_name == expected_name:
        counts["same_length_and_mnemonic"] += 1
    else:
        counts["needs_review"] += 1
        name = expected_name.upper()
        if actual_length == len(encoded) and (
            name in {"LDC.B", "LDC.W", "LDC.L", "STC.B", "STC.W", "STC.L",
                     "ADDX.B", "SUBX.B", "INC.B"} and actual_name == name.split(".")[0].lower()
        ):
            classification = "gnu_size_suffix_spelling"
        elif actual_length == len(encoded) and name in {"PUSH.W", "PUSH.L", "POP.W", "POP.L"} and actual_name == "mov." + name[-1].lower():
            # H8SX §2.2.85–88 define POP/PUSH as MOV to/from the stack.
            classification = "stack_move_alias"
        elif fill == 0 and name.split(".")[0] in {"ADD", "CMP", "MOV", "SUB"} and "#xx:3" in row["syntax"]:
            # Instruction-specific addressing matrices specify #x:3(1-7).
            classification = "reserved_zero_short_immediate"
        elif fill == 255 and name in {"BRA", "BRN", "BHI", "BLS", "BCC", "BCS",
                                     "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI",
                                     "BGE", "BLT", "BGT", "BLE"} and "d:16" in row["syntax"]:
            # §1.8.8 and §2.2.8 require even branch destinations.
            classification = "odd_branch_displacement"
        elif row.get("reviewed_value") and actual_name == ".word":
            classification = "printed_opcode_inconsistency"
        else:
            classification = "unresolved"
        classifications[classification] += 1
        mismatches.append(dict(classification=classification, row=index, page=row["page"], syntax=row["syntax"],
                               fill=fill, bytes=encoded.hex(), actual_length=actual_length,
                               actual=actual))
assert len(seen) == len(cases), (len(seen), len(cases))
assert counts == {"same_length_and_mnemonic": 16865, "needs_review": 121}, counts
assert classifications == {
    "printed_opcode_inconsistency": 2,
    "reserved_zero_short_immediate": 17,
    "gnu_size_suffix_spelling": 78,
    "odd_branch_displacement": 16,
    "stack_move_alias": 8,
}, classifications
assert inferred_checks == corrected_checks == 2
args.output.write_text(json.dumps(dict(counts=counts, classifications=classifications, inferred_and_assemble_back=inferred_checks, corrected_add_assemble_back=corrected_checks, mismatches=mismatches), indent=2) + "\n")
print(dict(counts))
print(dict(classifications))
