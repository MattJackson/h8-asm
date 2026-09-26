//! Independent, opt-in H8SX first-word length probe using GNU binutils.
//!
//! Run with H8_AS and H8_OBJDUMP set to an h8300-elf toolchain:
//! cargo test --test binutils_first_word -- --ignored --nocapture

use h8_asm::{
    isa::{insn_len, MAX_INSN_LEN},
    Mode, Target,
};
use std::{env, fs, process::Command};

#[test]
#[ignore = "requires an H8SX-capable h8300-elf assembler and objdump"]
fn h8sx_zero_suffix_first_word_sweep() {
    let assembler = env::var("H8_AS").unwrap_or_else(|_| "h8300-elf-as".into());
    let objdump = env::var("H8_OBJDUMP").unwrap_or_else(|_| "h8300-elf-objdump".into());
    let work = env::temp_dir().join(format!("h8-asm-binutils-{}", std::process::id()));
    fs::create_dir_all(&work).unwrap();
    let source = work.join("first-word.s");
    let object = work.join("first-word.o");

    // Every candidate starts at a 16-byte boundary. A zero suffix fills
    // the remainder of the maximum-length slot, so the next candidate
    // stays aligned even if binutils consumes extension words.
    let mut assembly = String::from(".h8300sx\n.text\n");
    for first in 0..=u16::MAX {
        assembly.push_str(&format!(".word 0x{first:04x}\n.space 14,0\n"));
    }
    assert_eq!(MAX_INSN_LEN, 14);
    fs::write(&source, assembly).unwrap();

    let assembled = Command::new(&assembler)
        .arg("-o")
        .arg(&object)
        .arg(&source)
        .output()
        .expect("run H8 assembler");
    assert!(
        assembled.status.success(),
        "{}",
        String::from_utf8_lossy(&assembled.stderr)
    );
    let disassembled = Command::new(&objdump)
        .args(["-d", "-z", "-w"])
        .arg(&object)
        .output()
        .expect("run H8 objdump");
    assert!(
        disassembled.status.success(),
        "{}",
        String::from_utf8_lossy(&disassembled.stderr)
    );
    let text = String::from_utf8(disassembled.stdout).unwrap();
    let mut checked = 0;
    let mut agree = 0;
    let mut both_reject = 0;
    let mut binutils_only = 0;
    let mut disagreements = Vec::new();
    let mut reverse = Vec::new();

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
        let encoded = match columns.next() {
            Some(encoded) => encoded,
            None => continue,
        };
        let mnemonic = match columns.next() {
            Some(mnemonic) => mnemonic,
            None => continue,
        };
        let byte_count = encoded.split_whitespace().count();
        let recognized = !mnemonic.trim_start().starts_with(".word");
        let first = offset / 16;
        let mut bytes = [0; MAX_INSN_LEN];
        bytes[..2].copy_from_slice(&(first as u16).to_be_bytes());
        let ours = insn_len(&bytes, Target::H8SX, Mode::Normal);
        checked += 1;
        match (ours, recognized) {
            (Some(length), true) if length == byte_count => agree += 1,
            (None, false) => both_reject += 1,
            (None, true) => {
                binutils_only += 1;
                reverse.push(format!(
                    "{first:04x}\t{byte_count}\t{mnemonic}\t{}",
                    columns.collect::<Vec<_>>().join(" ")
                ));
            }
            _ => disagreements.push(format!(
                "{first:04x}: ours {ours:?}, binutils {byte_count} {mnemonic}"
            )),
        }
    }

    let expected: Vec<String> = include_str!("data/sx_reverse.tsv")
        .lines()
        .map(|line| line.split('\t').take(3).collect::<Vec<_>>().join("\t"))
        .collect();
    let observed: Vec<String> = reverse
        .iter()
        .map(|line| line.split('\t').take(3).collect::<Vec<_>>().join("\t"))
        .collect();
    assert_eq!(
        observed, expected,
        "reviewed reverse-census allow-list changed"
    );
    fs::remove_dir_all(&work).unwrap();
    println!("agree={agree} both_reject={both_reject} binutils_only={binutils_only}");
    assert_eq!(checked, u16::MAX as usize + 1);
    assert_eq!((agree, both_reject, binutils_only), (57677, 7357, 502));
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}
