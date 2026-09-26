//! Opt-in GNU binutils assemble-back oracle for the typed H8SX subset.
//!
//! Set H8_AS, H8_LD, and H8_OBJCOPY to h8300-elf tools, then run:
//! cargo test --test binutils_sx_assemble_back -- --ignored --nocapture
//! GNU gas uses `.` for the Renesas `$` location counter and `0x` for `H'`.
//! All supported two-byte words and representative four-byte forms are
//! decoded, rendered, reassembled, and compared byte for byte.

use h8_asm::{
    isa::{sx_disasm, sx_encode, sx_semantic},
    Mode, Target,
};
use std::{env, fs, process::Command};

fn gas_syntax(rendered: &str) -> String {
    rendered.replace('$', ".").replace("H'", "0x")
}

fn append_case(assembly: &mut String, expected: &mut Vec<u8>, bytes: &[u8]) {
    let decoded = sx_semantic::decode(bytes, Target::H8SX, Mode::Maximum).expect("semantic decode");
    assert_eq!(decoded.len, bytes.len());
    let encoded = sx_encode::encode(decoded.instruction, Target::H8SX, Mode::Maximum)
        .expect("semantic encode");
    assert_eq!(encoded.as_bytes(), bytes, "{bytes:02x?}");
    assembly.push_str(&gas_syntax(&sx_disasm::disassemble(decoded.instruction)));
    assembly.push('\n');
    expected.extend_from_slice(bytes);
}

#[test]
#[ignore = "requires H8SX-capable GNU as, ld, and objcopy"]
fn h8sx_semantic_assemble_back() {
    let assembler = env::var("H8_AS").unwrap_or_else(|_| "h8300-elf-as".into());
    let linker = env::var("H8_LD").unwrap_or_else(|_| "h8300-elf-ld".into());
    let objcopy = env::var("H8_OBJCOPY").unwrap_or_else(|_| "h8300-elf-objcopy".into());
    let work = env::temp_dir().join(format!("h8-asm-sx-assemble-back-{}", std::process::id()));
    fs::create_dir_all(&work).unwrap();
    let source = work.join("cases.s");
    let object = work.join("cases.o");
    let executable = work.join("cases.elf");
    let binary = work.join("cases.bin");
    let mut assembly = String::from(".h8300sx\n.text\n.global _start\n_start:\n");
    let mut expected = Vec::new();
    let mut two_byte_cases = 0;
    let mut four_byte_cases = 0;

    for first in 0..=u16::MAX {
        let bytes = first.to_be_bytes();
        if sx_semantic::decode(&bytes, Target::H8SX, Mode::Maximum).is_some() {
            append_case(&mut assembly, &mut expected, &bytes);
            two_byte_cases += 1;
        }
    }
    // A zero suffix is insufficient for a complete four-byte semantic
    // census. These cases cover each form, every Bcc condition, both signed
    // branch limits, address extremes, and every byte-register code.
    for condition in 0..16u8 {
        for displacement in [-32768i16, -4, 0, 32766] {
            let [hi, lo] = displacement.to_be_bytes();
            append_case(
                &mut assembly,
                &mut expected,
                &[0x58, condition << 4, hi, lo],
            );
            four_byte_cases += 1;
        }
    }
    for displacement in [-32768i16, -4, 0, 32766] {
        let [hi, lo] = displacement.to_be_bytes();
        append_case(&mut assembly, &mut expected, &[0x5c, 0, hi, lo]);
        four_byte_cases += 1;
    }
    for address in [0u32, 0xffff, 0x4194de, 0xffffff] {
        let [_, a, b, c] = address.to_be_bytes();
        for opcode in [0x5a, 0x5e] {
            append_case(&mut assembly, &mut expected, &[opcode, a, b, c]);
            four_byte_cases += 1;
        }
    }
    for register in 0..16u8 {
        for address in [0u16, 0xffff] {
            let [hi, lo] = address.to_be_bytes();
            for direction in [0u8, 0x80] {
                append_case(
                    &mut assembly,
                    &mut expected,
                    &[0x6a, direction | register, hi, lo],
                );
                four_byte_cases += 1;
            }
        }
    }
    let mut extended_cases = 0;
    for opcode in 0..7u8 {
        for register in 0..16u8 {
            for immediate in [0u16, 1, 0x7fff, 0x8000, 0xffff] {
                let [a, b] = immediate.to_be_bytes();
                append_case(
                    &mut assembly,
                    &mut expected,
                    &[0x79, opcode << 4 | register, a, b],
                );
                extended_cases += 1;
            }
        }
        for register in 0..8u8 {
            for immediate in [
                0u32, 1, 0x7fff, 0x8000, 0xffff, 0x10000, 0x7fffffff, 0x80000000, 0xffffffff,
            ] {
                let [a, b, c, d] = immediate.to_be_bytes();
                append_case(
                    &mut assembly,
                    &mut expected,
                    &[0x7a, opcode << 4 | register, a, b, c, d],
                );
                extended_cases += 1;
            }
        }
    }
    assert_eq!(extended_cases, 1064);
    assert_eq!(two_byte_cases, 39297);
    assert_eq!(four_byte_cases, 140);
    fs::write(&source, assembly).unwrap();

    let assembled = Command::new(&assembler)
        .arg("-o")
        .arg(&object)
        .arg(&source)
        .output()
        .expect("run GNU as");
    assert!(
        assembled.status.success(),
        "{}",
        String::from_utf8_lossy(&assembled.stderr)
    );
    let linked = Command::new(&linker)
        .args(["-m", "h8300sxelf", "-o"])
        .arg(&executable)
        .arg(&object)
        .output()
        .expect("run GNU ld");
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let copied = Command::new(&objcopy)
        .args(["-O", "binary", "-j", ".text"])
        .arg(&executable)
        .arg(&binary)
        .output()
        .expect("run GNU objcopy");
    assert!(
        copied.status.success(),
        "{}",
        String::from_utf8_lossy(&copied.stderr)
    );

    let actual = fs::read(&binary).unwrap();
    assert_eq!(actual.len(), expected.len(), "assembled length changed");
    assert_eq!(actual, expected, "GNU assembled bytes differ");
    println!("checked {two_byte_cases} two-byte and {four_byte_cases} four-byte forms; {extended_cases} word/long immediate cases");
    fs::remove_dir_all(&work).unwrap();
}
