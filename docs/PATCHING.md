# Flat-image assembly, relocation, and hooks

All offsets are addresses. CPU target and mode are explicit. These APIs operate
on instruction families accepted by the reviewed length recognizer. Encoding
coverage does not establish safe relocation for every control-flow form.

## Labels

`Asm::new(target, mode)` creates a builder. `instruction(bytes)` accepts exactly
one recognized position-independent instruction. Use `branch(condition, label)`
and `call(label)` for relative control flow; `label(name)` binds the next
instruction or the block end. `finish(origin)` resolves all labels and returns
bytes. It rejects undefined/duplicate labels, invalid conditions, unsupported
CPU modes, and unrepresentable addresses/branches.

Relaxation begins with short branches and only widens. An earlier widening can
move another label or bring an external destination into range, so layout is
recomputed until stable. H8/300 has no long relative branch format and refuses
out-of-range branches. Later cores use the manual's four-byte Bcc/BSR forms.

## Relocation

`relocate::relocate(bytes, from, to, target, mode)` returns new bytes and an
original-to-relocated instruction offset map. It never modifies its input.
Relative targets inside the block map to the corresponding relocated
instruction; an interior-byte destination is refused. External targets,
including the source block end, retain their original addresses. Relative
arithmetic wraps at the mode's 16-, 24-, or 32-bit PC width, but a block itself
may not straddle the end of that address space.

BRA/S, PC-indexed branches, bit-test relative branches and MOVSD.B are refused. Absolute jumps/calls into the source
block are also refused: blindly copying them into a trampoline could jump back
into overwritten code. Ordinary recognized position-independent instructions
retain their exact bytes. No liveness or runtime register values are inferred.

## Transactional detours

`detour::plan(image, site, hook, trampoline, target, mode)` validates and returns
a reviewable plan without writing. `detour::tramp` performs the same planning
and then installs both ranges. Every returned error leaves the image unchanged.
There is no apply method accepting a mutable, caller-edited plan.

The hook site receives an absolute JMP to `hook`: four bytes in normal,
middle, or advanced mode; six bytes in H8SX maximum mode. The displaced block
ends on an instruction boundary. Remaining site bytes become NOP words. The
trampoline contains relocated instructions followed by an absolute JMP to the
first untouched original instruction.

All three entry addresses must be aligned, mode-valid, and inside the image.
Site/trampoline ranges and the hook entry may not overlap. The entire planned
trampoline area must contain erased bytes (`0xff`). A possible BRA/S immediately
before the site is conservatively rejected as a delay-slot hazard.

The caller must identify a genuine instruction boundary and rule out other
entry points into the overwritten interior. No local byte inspection can prove
that a register-indirect branch elsewhere will never enter that range. These
APIs check encoding and layout, not the behavior of the hook or the firmware.

## Search, tables, and analysis

`find`, `Needle`, and `find_free_space` support exact bytes, big-endian 32-bit
values, aligned masked instruction words, and aligned erased runs. Alignment is
checked during the free-space search. Empty patterns and zero alignment/length
produce no match. Checked reads return `Option`; writes/inserts return errors
without partial edits. Insertion requires erased bytes.

`CommandTable` describes fixed-size records with opcode/flags bytes and a
four-byte big-endian handler. It validates field geometry, bounds, and a
terminator before returning records or replacing a handler. Its pointer width
is a property of this supplied table format, not an assumption about all H8
vector tables.

`analysis::xrefs` walks a supplied code range and reports direct and unresolved
control-flow references. Memory-indirect vectors use two-byte entries in
normal mode and four-byte entries otherwise; middle/advanced program targets
use only the low 24 bits, as specified in H8SX §1.8.10–11. Missing vector data
and register/PC-indexed destinations remain explicitly unresolved.

`reachable` explores local branches, follows the return path of calls without
entering callees, and terminates at returns. It refuses unresolved jumps,
delay slots, unknown instructions, and overlapping instruction interpretations.
`function_start` selects a unique reachable entry from caller-supplied known
entries; it does not guess prologues by scanning backward.

## Evidence

Tests cover branch widening cascades, mode wraparound, interior-target refusal,
error atomicity, vector widths/endian order, malformed tables, and overlapping
control-flow paths. GNU assemble-back separately verifies planned H8SX hooks,
trampolines, and a label layout requiring repeated widening. These are encoding
and consistency checks, not execution tests on H8 hardware.
