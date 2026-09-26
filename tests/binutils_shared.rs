//! Independent assemble-back checks for the unified legacy vocabulary.
use h8_asm::{
    isa::{decode_insn, disassemble_insn, encode_insn, insn_len},
    Mode, Target,
};
use std::{env, fs, process::Command};

const CORES: [(Target, Mode, &str, &str, u32); 4] = [
    // GNU's default operand mode is H8/300 but its ELF machine header is
    // h8300h (tc-h8300.c: default_mach versus initially-zero Hmode).
    (Target::H8_300, Mode::Normal, "", "h8300helf", 51380),
    (
        Target::H8_300H,
        Mode::Advanced,
        ".h8300h",
        "h8300helf",
        54344,
    ),
    (
        Target::H8S2000,
        Mode::Advanced,
        ".h8300s",
        "h8300self",
        54696,
    ),
    (
        Target::H8S2600,
        Mode::Advanced,
        ".h8300s",
        "h8300self",
        54729,
    ),
];

fn compare(
    cases: Vec<Vec<u8>>,
    target: Target,
    mode: Mode,
    directive: &str,
    emulation: &str,
    label: &str,
) {
    let work = env::temp_dir().join(format!(
        "h8-shared-{label}-{target:?}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&work).unwrap();
    let mut source = format!("{directive}\n.text\n.global _start\n_start:\n");
    let mut expected = Vec::new();
    let mut restricted = Vec::new();
    let mut assembled_cases = Vec::new();
    for bytes in &cases {
        let decoded = decode_insn(bytes, target, mode).unwrap();
        assert_eq!(encode_insn(decoded.insn, target, mode).unwrap(), *bytes);
        let line = disassemble_insn(decoded.insn, target, mode)
            .unwrap()
            .replace('$', ".")
            .replace("H'", "0x");
        // The generic H8S software manual lists these ER7 groups, but
        // product hardware manuals prohibit them (e.g. H8S/2169 §2.10.2).
        // GNU refuses to assemble them. Check its disassembly separately;
        // do not count raw data directives as assemble-back evidence.
        if matches!(target, Target::H8S2000 | Target::H8S2600)
            && matches!(
                bytes.as_slice(),
                [1, 0x10, 0x6d, 0x77 | 0xf6] | [1, 0x30, 0x6d, 0x77 | 0xf4]
            )
        {
            restricted.push((bytes.clone(), line));
            continue;
        }
        assembled_cases.push(bytes);
        source.push_str(&line);
        source.push('\n');
        expected.extend_from_slice(bytes);
    }
    fs::write(work.join("cases.s"), source).unwrap();
    for (variable, default, args) in [
        ("H8_AS", "h8300-elf-as", vec!["-o", "cases.o", "cases.s"]),
        (
            "H8_LD",
            "h8300-elf-ld",
            vec!["-m", emulation, "-o", "cases.elf", "cases.o"],
        ),
        (
            "H8_OBJCOPY",
            "h8300-elf-objcopy",
            vec!["-O", "binary", "-j", ".text", "cases.elf", "cases.bin"],
        ),
    ] {
        let result = Command::new(env::var(variable).unwrap_or_else(|_| default.into()))
            .current_dir(&work)
            .args(args)
            .output()
            .expect("run oracle tool");
        fs::write(work.join(format!("{variable}.stderr")), &result.stderr).unwrap();
        assert!(
            result.status.success(),
            "{target:?} {variable}: {}",
            String::from_utf8_lossy(&result.stderr)
                .lines()
                .take(18)
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(
            result.stderr.is_empty(),
            "{target:?} {variable} warnings: {}",
            String::from_utf8_lossy(&result.stderr)
                .lines()
                .take(18)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    let actual = fs::read(work.join("cases.bin")).unwrap();
    assert_eq!(actual.len(), expected.len(), "{target:?} output length");
    let mut offset = 0;
    let mut aliases = 0;
    let mut gnu_jump_divergences = 0;
    for (index, &bytes) in assembled_cases.iter().enumerate() {
        let actual = &actual[offset..offset + bytes.len()];
        if actual != bytes {
            // H8S REJ09B0139 §2.4 Table 2.2 note 2 permits bit 7 in
            // byte 4 of the MOV.L displacement store. GNU chooses it set.
            let documented_alias = bytes.len() == 10
                && bytes[..3] == [1, 0, 0x78]
                && bytes[4] == 0x6b
                && bytes[5] & 0xf8 == 0xa0
                && actual[3] ^ bytes[3] == 0x80
                && actual
                    .iter()
                    .zip(bytes)
                    .enumerate()
                    .all(|(i, (a, b))| i == 3 || a == b);
            // GNU tc-h8300.c ABSJMP handling sign-extends a 16-bit
            // H8/300 address into the reserved high byte. ADE-602-025 §2
            // JMP/JSR fixes that byte to zero. Do not copy GNU's bytes.
            let gnu_base_jump = target == Target::H8_300
                && bytes.len() == 4
                && matches!(bytes[0], 0x5a | 0x5e)
                && bytes[1] == 0
                && bytes[2] & 0x80 != 0
                && actual[0] == bytes[0]
                && actual[1] == 0xff
                && actual[2..] == bytes[2..];
            assert!(
                documented_alias || gnu_base_jump,
                "{target:?} case {index}: actual {actual:02x?}, expected {bytes:02x?}"
            );
            if documented_alias {
                aliases += 1;
            } else {
                gnu_jump_divergences += 1;
            }
        }
        offset += bytes.len();
    }
    println!(
        "{target:?} {label}: {} cases, {} exact, {aliases} documented alias choices, {gnu_jump_divergences} GNU base-jump divergences",
        assembled_cases.len(),
        assembled_cases.len() - aliases - gnu_jump_divergences
    );
    if !restricted.is_empty() {
        assert_eq!(restricted.len(), 4);
        let mut source = format!("{directive}\n.text\n");
        for (bytes, _) in &restricted {
            source.push_str(&format!(
                ".byte {}\n",
                bytes
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        fs::write(work.join("restricted.s"), source).unwrap();
        let result = Command::new(env::var("H8_AS").unwrap_or_else(|_| "h8300-elf-as".into()))
            .current_dir(&work)
            .args(["-o", "restricted.o", "restricted.s"])
            .output()
            .unwrap();
        assert!(result.status.success() && result.stderr.is_empty());
        let result =
            Command::new(env::var("H8_OBJDUMP").unwrap_or_else(|_| "h8300-elf-objdump".into()))
                .current_dir(&work)
                .args(["-d", "-w", "restricted.o"])
                .output()
                .unwrap();
        assert!(result.status.success() && result.stderr.is_empty());
        let listing = String::from_utf8(result.stdout)
            .unwrap()
            .to_ascii_lowercase();
        let instructions = listing
            .lines()
            .filter_map(|line| {
                let (address, rest) = line.split_once(':')?;
                usize::from_str_radix(address.trim(), 16).ok()?;
                let mut columns = rest.split('\t').filter(|s| !s.trim().is_empty());
                columns.next()?;
                Some(columns.collect::<Vec<_>>().join(""))
            })
            .collect::<Vec<_>>();
        assert_eq!(instructions.len(), restricted.len(), "{listing}");
        for ((_, expected), actual) in restricted.iter().zip(instructions) {
            let compact = |s: &str| {
                s.to_ascii_lowercase()
                    .replace("sp", "er7")
                    .chars()
                    .filter(|c| !c.is_whitespace() && *c != '(' && *c != ')')
                    .collect::<String>()
            };
            assert_eq!(compact(&actual), compact(expected), "{listing}");
        }
        println!("{target:?}: 4 product-restricted ER7 groups checked by disassembly only");
    }
    fs::remove_dir_all(work).unwrap();
}

#[test]
#[ignore = "requires H8 GNU as, ld and objcopy"]
fn every_shared_two_byte_instruction_assembles_back() {
    for (target, mode, directive, emulation, census) in CORES {
        let cases = (0..=u16::MAX)
            .filter_map(|first| {
                let bytes = first.to_be_bytes();
                if decode_insn(&bytes, target, mode).is_ok() {
                    Some(bytes.to_vec())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(cases.len(), census as usize);
        compare(cases, target, mode, directive, emulation, "two-byte");
    }
}

#[test]
#[ignore = "requires H8 GNU as, ld and objcopy"]
fn shared_extended_instruction_fields_assemble_back() {
    for (target, mode, directive, emulation, _) in CORES {
        let mut cases = Vec::new();
        let mut add = |bytes: &[u8]| {
            if let Some(len) = insn_len(bytes, target, mode) {
                if len > 2 {
                    cases.push(bytes[..len].to_vec());
                }
            }
        };
        for first in 0..=u16::MAX {
            for suffix in [0u8, 0x7e, 0x80, 0xfe] {
                let mut bytes = [suffix; 14];
                bytes[..2].copy_from_slice(&first.to_be_bytes());
                add(&bytes);
            }
        }
        for first in [
            0x0100u16, 0x0110, 0x0120, 0x0130, 0x0140, 0x0141, 0x0160, 0x01c0, 0x01d0, 0x01e0,
            0x01f0, 0x7800, 0x7870, 0x7b5c, 0x7bd4, 0x7c00, 0x7d70, 0x7e12, 0x7fff,
        ] {
            for second in 0..=u16::MAX {
                let mut bytes = [0u8; 14];
                bytes[..2].copy_from_slice(&first.to_be_bytes());
                bytes[2..4].copy_from_slice(&second.to_be_bytes());
                add(&bytes);
            }
        }
        for prefix in [0x0100u16, 0x0140, 0x0141] {
            for base in 0..8u8 {
                for register in 0..8u8 {
                    for direction in [0u8, 0x80] {
                        for value in [0u32, 0x7fffff, 0x800000, 0xffffff, 0xffffffff] {
                            let mut bytes = prefix.to_be_bytes().to_vec();
                            bytes.extend_from_slice(&[
                                0x78,
                                base << 4,
                                0x6b,
                                0x20 | direction | register,
                            ]);
                            bytes.extend_from_slice(&value.to_be_bytes());
                            add(&bytes);
                            if prefix == 0x0100 {
                                bytes[3] |= 0x80;
                                add(&bytes);
                            }
                        }
                    }
                }
            }
        }
        for first in [0x6a10u16, 0x6a18, 0x6a30, 0x6a38] {
            for last in 0..=u16::MAX {
                let mut bytes = first.to_be_bytes().to_vec();
                if first & 0x20 == 0 {
                    bytes.extend_from_slice(&[0x80, 0x01]);
                } else {
                    bytes.extend_from_slice(&[0x80, 0, 0, 1]);
                }
                bytes.extend_from_slice(&last.to_be_bytes());
                add(&bytes);
            }
        }
        cases.sort();
        cases.dedup();
        compare(cases, target, mode, directive, emulation, "extended");
    }
}

#[test]
#[ignore = "requires H8 GNU as, ld and objcopy"]
fn legacy_normal_modes_assemble_back() {
    for (target, directive, emulation, expected) in [
        (Target::H8_300H, ".h8300hn", "h8300hnelf", 54344),
        (Target::H8S2000, ".h8300sn", "h8300snelf", 54696),
        (Target::H8S2600, ".h8300sn", "h8300snelf", 54729),
    ] {
        let cases: Vec<_> = (0..=u16::MAX)
            .filter_map(|word| {
                let bytes = word.to_be_bytes();
                decode_insn(&bytes, target, Mode::Normal)
                    .ok()
                    .map(|_| bytes.to_vec())
            })
            .collect();
        assert_eq!(cases.len(), expected);
        compare(
            cases,
            target,
            Mode::Normal,
            directive,
            emulation,
            "normal-two-byte",
        );
    }
}

#[test]
#[ignore = "requires H8 GNU as, ld and objcopy"]
fn inherited_longer_operand_fields_assemble_back() {
    for (target, mode, directive, emulation, _) in CORES.into_iter().skip(1) {
        let mut cases = Vec::new();
        for record in include_bytes!("data/sx_fields.bin").chunks_exact(15) {
            let bytes = &record[1..1 + usize::from(record[0])];
            if let Some(len) = insn_len(bytes, target, mode) {
                if len > 4 {
                    cases.push(bytes[..len].to_vec());
                }
            }
        }
        cases.sort();
        cases.dedup();
        compare(
            cases,
            target,
            mode,
            directive,
            emulation,
            "inherited-fields",
        );
    }
}
