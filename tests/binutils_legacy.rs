//! Opt-in, independent GNU binutils first-word probe for the older H8 cores.
//!
//! Run with `H8_AS` and `H8_OBJDUMP` pointing at an h8300-elf toolchain:
//! `cargo test --test binutils_legacy -- --ignored --nocapture`.
//! Binutils is a comparison implementation, not the encoding authority; consult
//! the Renesas manuals before changing a rule based on a discrepancy.

use h8_asm::{
    isa::{insn_len, MAX_INSN_LEN},
    Mode, Target,
};
use std::{env, fs, process::Command};

const CORES: [(&str, Target); 4] = [
    ("", Target::H8_300),
    (".h8300h", Target::H8_300H),
    (".h8300s", Target::H8S2000),
    (".h8300s", Target::H8S2600),
];

#[test]
#[ignore = "requires an h8300-elf assembler and objdump"]
fn legacy_zero_suffix_first_word_sweep() {
    let assembler = env::var("H8_AS").unwrap_or_else(|_| "h8300-elf-as".into());
    let objdump = env::var("H8_OBJDUMP").unwrap_or_else(|_| "h8300-elf-objdump".into());
    let work = env::temp_dir().join(format!("h8-asm-legacy-binutils-{}", std::process::id()));
    fs::create_dir_all(&work).unwrap();

    // Fourteen bytes of zeros after the first word leave every candidate on
    // a sixteen-byte boundary, even for the longest H8SX instruction.
    // This probes only one completion of each first word; prefixes with a
    // required nonzero subsequent opcode will appear rejected.
    assert_eq!(MAX_INSN_LEN, 14);
    for (index, (directive, target)) in CORES.iter().enumerate() {
        let source = work.join(format!("core-{index}.s"));
        let object = work.join(format!("core-{index}.o"));
        let mut assembly = format!("{directive}\n.text\n");
        for first in 0..=u16::MAX {
            assembly.push_str(&format!(".word 0x{first:04x}\n.space 14,0\n"));
        }
        fs::write(&source, assembly).unwrap();

        let assembled = Command::new(&assembler)
            .arg("-o")
            .arg(&object)
            .arg(&source)
            .output()
            .expect("run H8 assembler");
        assert!(
            assembled.status.success(),
            "{target:?}: {}",
            String::from_utf8_lossy(&assembled.stderr)
        );
        let disassembled = Command::new(&objdump)
            .args(["-d", "-z", "-w"])
            .arg(&object)
            .output()
            .expect("run H8 objdump");
        assert!(
            disassembled.status.success(),
            "{target:?}: {}",
            String::from_utf8_lossy(&disassembled.stderr)
        );

        let text = String::from_utf8(disassembled.stdout).unwrap();
        let mut checked = 0;
        let mut agree = 0;
        let mut both_reject = 0;
        let mut binutils_only = 0;
        let mut ours_only = 0;
        let mut length_diff = 0;
        let mut binutils_examples = Vec::new();
        let mut ours_examples = Vec::new();
        let mut length_examples = Vec::new();
        for line in text.lines() {
            let (address, columns) = match line.split_once(':') {
                Some(parts) => parts,
                None => continue,
            };
            let offset = match usize::from_str_radix(address.trim(), 16) {
                Ok(offset) => offset,
                Err(_) => continue,
            };
            if offset % 16 != 0 || offset / 16 > u16::MAX as usize {
                continue;
            }
            let mut columns = columns.split('\t').filter(|field| !field.trim().is_empty());
            let encoded = columns.next().expect("instruction bytes");
            let mnemonic = columns.next().expect("mnemonic");
            let binutils_len = encoded.split_whitespace().count();
            let binutils_recognized = !mnemonic.trim_start().starts_with(".word");
            let first = offset / 16;
            let mut bytes = [0; MAX_INSN_LEN];
            bytes[..2].copy_from_slice(&(first as u16).to_be_bytes());
            let ours = insn_len(&bytes, *target, Mode::Normal);
            checked += 1;
            let example = format!("{first:04x}: ours {ours:?}, binutils {binutils_len} {mnemonic}");
            match (ours, binutils_recognized) {
                (Some(len), true) if len == binutils_len => agree += 1,
                (None, false) => both_reject += 1,
                (None, true) => {
                    binutils_only += 1;
                    if binutils_examples.len() < 8 {
                        binutils_examples.push(example);
                    }
                }
                (Some(_), false) => {
                    ours_only += 1;
                    if ours_examples.len() < 8 {
                        ours_examples.push(example);
                    }
                }
                (Some(_), true) => {
                    length_diff += 1;
                    if length_examples.len() < 8 {
                        length_examples.push(example);
                    }
                }
            }
        }
        assert_eq!(checked, u16::MAX as usize + 1, "{target:?}");
        println!(
            "{target:?}: agree={agree} both_reject={both_reject} binutils_only={binutils_only} ours_only={ours_only} length_diff={length_diff}"
        );
        for example in binutils_examples
            .into_iter()
            .chain(ours_examples)
            .chain(length_examples)
        {
            println!("  {example}");
        }
    }
    fs::remove_dir_all(&work).unwrap();
}
