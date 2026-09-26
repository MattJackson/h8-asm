# Mutation audit — 2026-09-26

The combined audit has **1,808 mutations: 1,698 caught, 67 surviving, five
timeouts, and 38 unviable**. This is not a zero-survivor result. Mutation
testing supplements the independent conformance and exhaustive decode tests;
it does not prove the absence of bugs.

The machine-readable [audit](mutation-audit.json) records every mutation,
outcome, survivor review category, and SHA-256 of the reviewed source/tests.
Locations in mutation names identify the audited source, not future revisions.

## Runs and reconciliation

The full baseline used cargo-mutants 27.1.0 on an isolated Ubuntu 24.04
c7i.8xlarge EC2 worker:

```text
cargo mutants --profile release -j 16 --jobserver-tasks 32 --timeout 180 --build-timeout 180
```

The source archive SHA-256 was
`e2a21690d16097fe824f14ad078251194f1471da591f1c0c108693cc4caa3c6c`.
The run completed at 18:24 UTC: 1,801 mutations, 1,633 caught, 126 surviving,
four timeouts, 38 unviable. Results were retrieved before terminating the
worker and deleting its temporary security group and SSH key pair.

Regression rechecks used the release profile on macOS, six jobs, and
120-second build/test timeouts. The first batches caught 28, 12 and 12
previous survivors. A two-mutation canonical-encoding check caught both
(one previously survived). The final 72-survivor batch caught six and left
66. All original surviving mutations outside analysis were rechecked.
Run an individual mutation with the same options and `--re` matching its
escaped full name from the JSON file.

The vector-alignment correction changed analysis.rs. Its complete final
audit replaces all 82 original analysis mutations with 89: 81 caught, one
surviving, one timeout and six unviable. Other source files were unchanged;
the regression suites added assertions. Rechecks override baseline outcomes
instead of being counted as additional mutations. This produces the 1,808
distinct mutations above. Package version changes do not change this code.

## Survivor review

Each of the 67 survivors is assigned one of these categories in the JSON.
These equivalence assessments apply to the public API and current validated
call paths; they are not exclusions from future mutation runs.

| Category | Count | Why the mutation does not change the public result |
|---|---:|---|
| `disjoint_bits` | 31 | OR changed to XOR between disjoint fields. Register/condition checks restrict values to their low nibble (or three bits for long registers); opcode, direction and inversion bits occupy other positions. This covers the subset encoders, SX register encoder and legacy bit/pair encoders. |
| `cleared_field` | 1 | `Field::write` clears the destination mask before inserting a value masked to those same bits. The two operands cannot overlap, so OR and XOR agree. |
| `legacy_semantic_validation` | 29 | Weakened internal register/range/size/address-kind or alias guards can produce an extra candidate, but `codec::encode_insn` decodes it and compares the entire instruction, widths, operands, encoding and length. A truncated register, wrong size, addressing direction or alias fails that equality and returns the same public error. Valid requests retain their original candidate. The alias guard mutations only remove a redundant refusal; the final comparison still requires `DisplacementStoreAlias`. |
| `sx_semantic_validation` | 3 | Weakened fixed-register, constant or mnemonic/size guards are followed by `candidate` checking `row.accepts(input)` and `spec.read(input) == insn`, then by full encode/decode equality. The guard cannot admit a changed public instruction. |
| `downstream_modulo` | 2 | Replacing the initial modulo with addition of the PC modulus leaves a congruent value; `displacement`/`flow` subsequently reduces it modulo the same width. Intermediates fit their integer types, and truncating the maximum-mode u32 destination preserves the same congruence. |
| `validated_legacy_selector` | 1 | Removing the H8SX guard for the six-byte absolute jump/call selector has no effect: callers already decoded the instruction. A valid legacy instruction cannot reach that selector. |

The semantic-validation cases retain their defensive guards. Deleting useful
validation simply to reduce the survivor count would not improve assurance.

## Behavioral gaps corrected

New assertions cover image search starts and write offsets; adjacent detour
ranges and exact image/mode ends; assembly condition 15; relocation's
exclusive source end; absolute-jump and immediate widths; reserved legacy
prefix fields; explicit versus implicit render counts; H8SX signed d8/d16
offsets and register-group endpoints; and canonical construction from every
SX row witness. The legacy prefix census is now pinned rather than printed.

Separately, reviewing vector behavior against the manuals corrected analysis:
legacy vector-table addresses ignore bit zero, whereas an odd H8SX branch-table
address remains unresolved. Regression tests cover the applicable modes.

## Timeouts and unviable mutations

The five timeouts change advancing offsets or branch-relaxation termination:
detour length addition becomes multiplication, analysis offset addition becomes
multiplication, and three layout mutations prevent reaching a stable layout.
They are detected nontermination cases, recorded separately from caught tests.
The 38 unviable mutations fail to compile; they are not counted as successful
behavioral detections. Future changes should rerun the audit and review new
survivors rather than carrying these classifications forward unchecked.

After this mutation snapshot, the ignored GNU older-core oracle was strengthened
to assert all counts and the complete reviewed reverse sets. It passed on
Rust 1.58 with pinned binutils. This adds independent conformance assertions,
not additional mutation results; the JSON retains the original audit hashes.
