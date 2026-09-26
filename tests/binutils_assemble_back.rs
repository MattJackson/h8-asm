//! Assemble-back oracle for every semantically decoded H8/300 two-byte word.
//!
//! Requires H8_AS, H8_LD and H8_OBJCOPY, or matching h8300-elf tools on PATH.
//! Run with `cargo test --test binutils_assemble_back -- --ignored --nocapture`.

use h8_asm::{
    isa::{decode::decode, disasm::disassemble, encode::encode},
    Mode, Target,
};
use std::{env, fs, process::Command};

fn run(program: &str, args: &[&str]) {
    let output = Command::new(program)
        .args(args)
        .output()
        .expect("run binutils");
    assert!(
        output.status.success(),
        "{}: {}",
        program,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires h8300-elf assembler, linker, and objcopy"]
fn every_decoded_h8_300_word_assembles_back_to_same_bytes() {
    let assembler = env::var("H8_AS").unwrap_or_else(|_| "h8300-elf-as".into());
    let linker = env::var("H8_LD").unwrap_or_else(|_| "h8300-elf-ld".into());
    let objcopy = env::var("H8_OBJCOPY").unwrap_or_else(|_| "h8300-elf-objcopy".into());
    let work = env::temp_dir().join(format!("h8-asm-assemble-back-{}", std::process::id()));
    fs::create_dir_all(&work).unwrap();
    let source = work.join("roundtrip.s");
    let object = work.join("roundtrip.o");
    let linked = work.join("roundtrip.elf");
    let binary = work.join("roundtrip.bin");
    let mut assembly = String::from(".text\n");
    let mut expected = Vec::new();
    let mut words = 0;

    for word in 0..=u16::MAX {
        let bytes = word.to_be_bytes();
        if let Ok(decoded) = decode(&bytes, Target::H8_300, Mode::Normal) {
            assert_eq!(
                encode(decoded.instruction, Target::H8_300, Mode::Normal),
                Ok(bytes)
            );
            // Renesas uses `$` for the current instruction; GNU gas uses
            // `.`. Branch expressions otherwise retain their displacement
            // and explicit eight-bit width. GNU gas leaves PC-relative
            // relocations in the object, so the linker must resolve them.
            assembly.push_str(&disassemble(decoded.instruction).replace('$', "."));
            assembly.push('\n');
            expected.extend_from_slice(&bytes);
            words += 1;
        }
    }
    fs::write(&source, assembly).unwrap();
    run(
        &assembler,
        &["-o", object.to_str().unwrap(), source.to_str().unwrap()],
    );
    run(
        &linker,
        &[
            "--entry=0",
            "-Ttext",
            "0",
            "-o",
            linked.to_str().unwrap(),
            object.to_str().unwrap(),
        ],
    );
    run(
        &objcopy,
        &[
            "-O",
            "binary",
            "--only-section=.text",
            linked.to_str().unwrap(),
            binary.to_str().unwrap(),
        ],
    );
    let actual = fs::read(&binary).unwrap();
    fs::remove_dir_all(&work).unwrap();
    assert_eq!(words, 2692);
    assert_eq!(actual.len(), expected.len());
    if let Some(index) = actual.iter().zip(expected.iter()).position(|(a, b)| a != b) {
        panic!(
            "first assemble-back mismatch at word {}: expected {:02x}{:02x}, got {:02x}{:02x}",
            index / 2,
            expected[index & !1],
            expected[index | 1],
            actual[index & !1],
            actual[index | 1]
        );
    }
    println!("{words} H8/300 instructions assembled back to exact bytes");
}
