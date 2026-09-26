//! Independent H8SX shared semantic assembly, including all manual-row witnesses.
use h8_asm::{
    isa::{decode_insn, disassemble_insn, Ea, Encoding, Operand, Reg},
    Mode, Target,
};
use std::{collections::BTreeMap, env, fs, process::Command};

fn compare(
    mut cases: Vec<Vec<u8>>,
    label: &str,
    expected_counts: (usize, usize, usize),
    mode: Mode,
) {
    cases.sort();
    cases.dedup();
    let work = env::temp_dir().join(format!("h8-sx-shared-{label}-{}", std::process::id()));
    fs::create_dir_all(&work).unwrap();
    let mut disassembly_only = BTreeMap::new();
    let directive = if mode == Mode::Normal {
        ".h8300sxn"
    } else {
        ".h8300sx"
    };
    let emulation = if mode == Mode::Normal {
        "h8300sxnelf"
    } else {
        "h8300sxelf"
    };
    let mut source = format!("{directive}\n.text\n.global _start\n_start:\n");
    for (index, bytes) in cases.iter().enumerate() {
        let decoded = decode_insn(bytes, Target::H8SX, mode).unwrap();
        let mut text = disassemble_insn(decoded.insn, Target::H8SX, mode)
            .unwrap()
            .replace("H'", "0x")
            .replace('$', ".");
        if matches!(
            decoded.insn.operands[0],
            Operand::Address(Ea::ExtendedIndirect(_))
        ) {
            text = text.replace(":7", "");
        }
        // gas get_mova_operands incorrectly compares register numbers with
        // &7, discarding RnH versus RnL / En versus Rn (manual §2.4 p.889).
        // The full form remains independently checkable by objdump.
        if decoded.insn.mnemonic.starts_with("MOVA/") {
            if let (
                Operand::Address(Ea::Indexed {
                    index: reg,
                    value,
                    bits,
                }),
                Operand::Register(Reg::Long(destination)),
            ) = (decoded.insn.operands[0], decoded.insn.operands[1])
            {
                let register = match reg {
                    Reg::Byte(n) if n < 8 && n == destination => Some(format!("r{n}h.b")),
                    Reg::Word(n) if n >= 8 && n - 8 == destination => Some(format!("e{}.w", n - 8)),
                    _ => None,
                };
                if let Some(register) = register {
                    let mask = if bits == 16 { 0xffff } else { u32::MAX };
                    disassembly_only.insert(
                        index,
                        format!(
                            "{}.l @(0x{:x}:{bits},{register}),er{destination}",
                            decoded.insn.mnemonic.to_ascii_lowercase(),
                            value as u32 & mask
                        ),
                    );
                }
            }
        }
        if matches!(decoded.insn.mnemonic, "SHLL" | "SHLR")
            && decoded.insn.operands[0] == (Operand::Immediate { value: 0, bits: 5 })
        {
            // §2.2.99 [8] permits zero, but GNU constant_fits_size L_5
            // requires >=1. Decode raw bytes independently instead.
            disassembly_only.insert(index, text.replace("#0:5", "#0").to_ascii_lowercase());
        }
        if disassembly_only.contains_key(&index) {
            text = format!(
                ".byte {}",
                bytes
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        source.push_str(&format!("_case_{index}:\n{text}\n"));
    }
    source.push_str("_case_end:\n");
    fs::write(work.join("cases.s"), &source).unwrap();
    let run = |variable: &str, default: &str, args: &[&str]| {
        let result = Command::new(env::var(variable).unwrap_or_else(|_| default.into()))
            .current_dir(&work)
            .args(args)
            .output()
            .unwrap();
        fs::write(work.join(format!("{variable}.stderr")), &result.stderr).unwrap();
        assert!(
            result.status.success(),
            "{label} {variable} ({work:?}): {}",
            String::from_utf8_lossy(&result.stderr)
                .lines()
                .take(25)
                .collect::<Vec<_>>()
                .join("\n")
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        for line in stderr
            .lines()
            .filter(|line| !line.is_empty() && !line.ends_with("Assembler messages:"))
        {
            let parts = line.splitn(3, ':').collect::<Vec<_>>();
            assert_eq!(variable, "H8_AS", "{line}");
            assert_eq!(parts.len(), 3, "{line}");
            assert!(
                parts[2]
                    .trim()
                    .starts_with("Warning: branch operand has odd offset"),
                "{line}"
            );
            let lineno = parts[1].parse::<usize>().unwrap();
            let statement = source.lines().nth(lineno - 1).unwrap();
            assert!(
                statement.starts_with("JMP @")
                    || statement.starts_with("JSR @")
                    || statement.starts_with("MOVSD.B "),
                "{line}: {statement}"
            );
            assert_eq!(lineno % 2, 0);
            let original = decode_insn(&cases[(lineno - 6) / 2], Target::H8SX, mode)
                .unwrap()
                .insn;
            assert!(
                original.operands.iter().any(|operand| match operand {
                    Operand::Address(Ea::Absolute { value, .. }) => value & 1 == 1,
                    Operand::Address(Ea::PcRelative { value, .. }) => value & 1 == 1,
                    _ => false,
                }),
                "warning without an odd target: {line}"
            );
        }
        result.stdout
    };
    run("H8_AS", "h8300-elf-as", &["-o", "cases.o", "cases.s"]);
    run(
        "H8_LD",
        "h8300-elf-ld",
        &["-m", emulation, "-Ttext=0", "-o", "cases.elf", "cases.o"],
    );
    run(
        "H8_OBJCOPY",
        "h8300-elf-objcopy",
        &["-O", "binary", "-j", ".text", "cases.elf", "cases.bin"],
    );
    let symbols = run("H8_OBJDUMP", "h8300-elf-objdump", &["-t", "cases.elf"]);
    let mut offsets = BTreeMap::new();
    for line in String::from_utf8(symbols).unwrap().lines() {
        let words = line.split_whitespace().collect::<Vec<_>>();
        if let Some(name) = words.last() {
            if let Some(index) = name.strip_prefix("_case_") {
                let index = if index == "end" {
                    cases.len()
                } else {
                    index.parse().unwrap()
                };
                offsets.insert(index, usize::from_str_radix(words[0], 16).unwrap());
            }
        }
    }
    assert_eq!(offsets.len(), cases.len() + 1);
    let actual = fs::read(work.join("cases.bin")).unwrap();
    if !disassembly_only.is_empty() {
        let listing = String::from_utf8(run(
            "H8_OBJDUMP",
            "h8300-elf-objdump",
            &["-d", "-z", "-w", "cases.elf"],
        ))
        .unwrap();
        for (&index, expected) in &disassembly_only {
            let at = offsets[&index];
            let text = listing
                .lines()
                .find_map(|line| {
                    let (address, rest) = line.split_once(':')?;
                    if usize::from_str_radix(address.trim(), 16).ok()? != at {
                        return None;
                    }
                    let mut cols = rest.split('\t').filter(|s| !s.trim().is_empty());
                    cols.next()?;
                    Some(cols.collect::<Vec<_>>().join(""))
                })
                .unwrap();
            let normalize = |s: &str| {
                s.replace("#0x0", "#0")
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>()
            };
            assert_eq!(normalize(&text), normalize(expected));
        }
    }
    let mut aliases = 0;
    let mut differences = Vec::new();
    for (index, expected) in cases.iter().enumerate() {
        let actual = &actual[offsets[&index]..offsets[&(index + 1)]];
        if disassembly_only.contains_key(&index) {
            continue;
        }
        if actual != expected {
            let original = decode_insn(expected, Target::H8SX, mode).unwrap().insn;
            let mut canonical = original;
            canonical.encoding = Encoding::Standard;
            let decoded = decode_insn(actual, Target::H8SX, mode);
            // Only a recorded alternative form may use meaning comparison.
            if matches!(original.encoding, Encoding::SxAlternative(_))
                && decoded.map(|d| d.insn) == Ok(canonical)
            {
                aliases += 1;
            } else {
                differences.push(format!(
                    "case {index}: {original:?}: {expected:02x?} -> {actual:02x?}"
                ));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "{label}: {}\nwork: {work:?}",
        differences
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        (
            cases.len() - aliases - disassembly_only.len(),
            aliases,
            disassembly_only.len()
        ),
        expected_counts
    );
    println!(
        "{label}: {} exact, {aliases} recorded alternative forms, {} disassembly-only",
        cases.len() - aliases - disassembly_only.len(),
        disassembly_only.len()
    );
    fs::remove_dir_all(work).unwrap();
}

#[test]
#[ignore = "requires H8SX GNU as, ld, objcopy and objdump"]
fn every_shared_sx_two_byte_word_assembles_back() {
    let cases = (0..=u16::MAX)
        .filter_map(|word| {
            let bytes = word.to_be_bytes();
            decode_insn(&bytes, Target::H8SX, Mode::Maximum)
                .ok()
                .map(|_| bytes.to_vec())
        })
        .collect::<Vec<_>>();
    assert_eq!(cases.len(), 56080);
    compare(cases, "two-byte", (56080, 0, 0), Mode::Maximum);
}

#[test]
#[ignore = "requires H8SX GNU as, ld, objcopy and objdump"]
fn every_shared_sx_manual_row_assembles_back() {
    let mut cases = Vec::new();
    for record in include_bytes!("data/sx_rows.bin").chunks_exact(71) {
        for start in [1, 15, 29, 43, 57] {
            cases.push(record[start..start + usize::from(record[0])].to_vec());
        }
    }
    compare(cases, "manual-rows", (42315, 12, 18), Mode::Maximum);
}

#[test]
#[ignore = "requires H8SX GNU as, ld, objcopy and objdump"]
fn shared_sx_extended_operand_fields_assemble_back() {
    let cases = include_bytes!("data/sx_fields.bin")
        .chunks_exact(15)
        .map(|record| record[1..1 + usize::from(record[0])].to_vec())
        .collect();
    compare(cases, "operand-fields", (163435, 18, 18), Mode::Maximum);
}

#[test]
#[ignore = "requires H8SX GNU as, ld, objcopy and objdump"]
fn shared_sx_normal_mode_assembles_back() {
    let cases = (0..=u16::MAX)
        .filter_map(|word| {
            let bytes = word.to_be_bytes();
            decode_insn(&bytes, Target::H8SX, Mode::Normal)
                .ok()
                .map(|_| bytes.to_vec())
        })
        .collect();
    compare(cases, "normal-two-byte", (56080, 0, 0), Mode::Normal);
    let mut cases = Vec::new();
    for record in include_bytes!("data/sx_rows.bin").chunks_exact(71) {
        for start in [1, 15, 29, 43, 57] {
            cases.push(record[start..start + usize::from(record[0])].to_vec());
        }
    }
    compare(cases, "normal-manual-rows", (42315, 12, 18), Mode::Normal);
}
