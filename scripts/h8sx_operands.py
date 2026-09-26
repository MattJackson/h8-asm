"""Compile H8SX manual operand notation into typed Rust descriptors."""
import re

SIZES = {"B": "Byte", "W": "Word", "L": "Long"}
ZERO = "Field(0,0,0)"


def split_operands(text):
    result, item, depth = [], "", 0
    for char in text + ",":
        depth += (char == "(") - (char == ")")
        if char == "," and depth == 0:
            if item:
                result.append(item)
            item = ""
        else:
            item += char
    assert depth == 0 and not item, text
    return result


class Compiler:
    def __init__(self, row):
        self.row = row
        syntax = row.get("inferred_syntax", row["syntax"]).replace("*", "")
        name, _, operands = syntax.partition(" ")
        parts = name.rsplit(".", 1)
        self.name = parts[0]
        self.size = parts[1] if len(parts) == 2 else None
        if self.name in {"ADDS", "SUBS"}:
            self.size = "L"
        if self.name in {"MOVFPE", "MOVTPE"}:
            self.size = "B"
        self.operands = split_operands(operands)
        self.fields = {}
        for token, offset, width in row["fields"]:
            self.fields.setdefault(token, []).append((offset, width))
        self.used = set()

    def field(self, token, width=None, scale=0, consume=False):
        available = self.fields.get(token, [])
        assert available, (self.row["syntax"], token)
        self.used.add(token)
        runs = []
        for offset, count in available:
            if runs and runs[-1][0] + runs[-1][1] == offset:
                runs[-1][1] += count
            else:
                runs.append([offset, count])
        width = width if width is not None else sum(n for _, n in runs)
        chosen = next((i for i, (_, n) in enumerate(runs) if n >= width), None)
        assert chosen is not None, (self.row["syntax"], token, width, runs)
        start, count = runs[chosen]
        if consume:
            runs[chosen] = [start+width, count-width]
            self.fields[token] = [(off,n) for off,n in runs if n]
        return f"Field({128-start-width},{width},{scale})"

    def reg(self, text, size=None, byte_index=False):
        if text in {"CCR", "EXR", "MACH", "MACL", "VBR", "SBR"}:
            reg = {"CCR": "Ccr", "EXR": "Exr", "MACH": "Mach",
                   "MACL": "Macl", "VBR": "Vbr", "SBR": "Sbr"}[text]
            return f"Register(Reg::{reg},{ZERO})"
        if text == "SP":
            return f"Register(Reg::Long(7),{ZERO})"
        match = re.fullmatch(r"(ER|R)([sdnm])(?:\.([BWL]))?", text)
        assert match, (self.row["syntax"], text)
        prefix, letter, suffix = match.groups()
        token = "r" + letter
        if token not in self.fields and "r" in self.fields:
            token = "r"
        size = suffix or ("L" if prefix == "ER" else size or "B")
        base = 8 if size == "B" and byte_index and sum(n for _, n in self.fields[token]) == 3 else 0
        kind = {"B": "Byte", "W": "Word", "L": "Long"}[size]
        return f"Register(Reg::{kind}({base}),{self.field(token)})"

    def address(self, text, size):
        kind = None
        register = f"Register(Reg::Long(0),{ZERO})"
        value, bits = ZERO, 0
        if text.startswith("@@"):
            bits = int(text.rsplit(":", 1)[1])
            token = "v" if "vec" in text else "a"
            value = self.field(token, bits, consume=True)
            kind = "ExtendedIndirect" if bits == 7 else "MemoryIndirect"
        elif text.startswith("@aa:"):
            bits = int(text.split(":")[1])
            value = self.field("a", bits, consume=True)
            kind = "Absolute"
        elif re.fullmatch(r"(?:ER|R)n\.[BWL]", text):
            register = self.reg(text, byte_index=True)
            kind = "PcIndexed"
        elif text.startswith("d:"):
            bits = int(re.match(r"d:(\d+)", text)[1])
            width = sum(n for _, n in self.fields["d"])
            scale = 1 if bits == 8 and width == 7 else 0
            value = self.field("d", bits-scale, scale, consume=True)
            kind = "PcRelative"
        elif text.startswith("@("):
            displacement, reg = split_operands(text[2:-1])
            bits = int(displacement.split(":")[1])
            value = self.field("d", bits, {"B": 0, "W": 1, "L": 2}[size] if bits == 2 else 0, consume=True)
            indexed = "." in reg
            register = self.reg(reg, byte_index=indexed)
            kind = "Indexed" if indexed else "Displacement"
        else:
            match = re.fullmatch(r"@([+-]?)(ER[sdnm]|SP)([+-]?)", text)
            assert match, (self.row["syntax"], text)
            before, reg, after = match.groups()
            register = self.reg(reg)
            kind = {("", ""): "Indirect", ("", "+"): "PostIncrement",
                    ("", "-"): "PostDecrement", ("+", ""): "PreIncrement",
                    ("-", ""): "PreDecrement"}[(before, after)]
        return f"AddressSpec {{ kind: AddressKind::{kind}, register: {register}, value: {value}, bits: {bits} }}"

    def operand(self, text, index):
        if text.startswith("@(d:") and ",@" in text:
            # MOVA's source displacement is encoded before its outer
            # displacement. Compile the inner memory access first.
            outer, inner = split_operands(text[2:-1])
            access = inner[-1]
            assert inner[-2] == "." and access in ("B", "W"), text
            address = self.address(inner[:-2], access)
            bits = int(outer.split(":")[1])
            value = self.field("d", bits, consume=True)
            return f"Template::IndexedMemory({address},Size::{SIZES[access]},{value},{bits})"
        if text.startswith("@") or text.startswith("d:") or re.fullmatch(r"(?:ER|R)n\.[BWL]", text):
            return f"Template::Address({self.address(text,self.size or 'B')})"
        if text.startswith("#"):
            if ":" in text:
                bits = int(text.split(":")[1])
                token = "x"
                return f"Template::Immediate({self.field(token,bits,consume=True)},{bits})"
            return f"Template::Constant({int(text[1:])})"
        if text.startswith("(ERn-"):
            count = int(text[-2])
            last = self.name != "STM"
            token = f"rn+{count}" if last else "rn"
            return f"Template::Registers({self.field(token)},{count},{str(last).lower()})"
        size = self.size or "B"
        if text == "Rn":
            size = "B"  # shift count or bit-number register
        if index == 1 and self.name in {"MULXS", "MULXU", "DIVXS", "DIVXU"}:
            size = {"B": "W", "W": "L"}[size]
        return f"Template::Register({self.reg(text,size)})"

    def compile(self):
        ops = [self.operand(op, i) for i, op in enumerate(self.operands)]
        if self.name in {"SHLL", "SHLR", "SHAL", "SHAR", "ROTL", "ROTR", "ROTXL", "ROTXR"} and len(ops) == 1:
            ops.insert(0, "Template::Constant(1)")
        assert len(ops) <= 3
        assert set(self.fields) == self.used, (self.row["syntax"], set(self.fields)-self.used)
        assert all(not self.fields.get(t) for t in ["d","a","x","v"]), self.row["syntax"]
        ops += ["Template::None"] * (3-len(ops))
        size = f"Some(Size::{SIZES[self.size]})" if self.size else "None"
        return f'Spec {{ mnemonic: "{self.name}", size: {size}, operands: [{",".join(ops)}] }}'
