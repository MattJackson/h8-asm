//! Print a bounded H8SX instruction walk and known control-flow edges.
//! Usage: cargo run --example sx_probe -- IMAGE LOAD_BASE START COUNT
//! All numeric arguments are hexadecimal (with or without 0x).
use h8_asm::{
    isa::{
        insn_len,
        sx_semantic::{decode, SxInstruction},
    },
    Mode, Target,
};
use std::{env, fs};

fn number(s: &str) -> usize {
    usize::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).expect("hex number")
}

fn relative_target(pc: usize, len: usize, displacement: i16) -> Option<usize> {
    usize::try_from(pc as i128 + len as i128 + displacement as i128).ok()
}

fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(
        args.len(),
        5,
        "usage: sx_probe IMAGE LOAD_BASE START COUNT (hex)"
    );
    let bytes = fs::read(&args[1]).expect("image");
    let base = number(&args[2]);
    let mut pc = number(&args[3]);
    let count = number(&args[4]);
    for _ in 0..count {
        let off = match pc.checked_sub(base) {
            Some(off) => off,
            None => break,
        };
        let tail = match bytes.get(off..) {
            Some(tail) => tail,
            None => break,
        };
        let len = match insn_len(tail, Target::H8SX, Mode::Maximum) {
            Some(len) => len,
            None => {
                println!("{pc:08x}: unknown length; stopping");
                break;
            }
        };
        let code = &tail[..len];
        let semantic = decode(code, Target::H8SX, Mode::Maximum);
        let target = semantic.and_then(|d| match d.instruction {
            SxInstruction::Branch8 { displacement, .. }
            | SxInstruction::BranchSubroutine8 { displacement } => {
                relative_target(pc, len, displacement as i16)
            }
            SxInstruction::Branch16 { displacement, .. }
            | SxInstruction::BranchSubroutine16 { displacement } => {
                relative_target(pc, len, displacement)
            }
            SxInstruction::Jump24 { address } | SxInstruction::Call24 { address } => {
                Some(address as usize)
            }
            _ => None,
        });
        let hex: String = code.iter().map(|b| format!("{b:02x}")).collect();
        match target {
            Some(t) => println!(
                "{pc:08x}: {hex:<28} {:?} -> {t:08x}",
                semantic.unwrap().instruction
            ),
            None => match semantic {
                Some(d) => println!("{pc:08x}: {hex:<28} {:?}", d.instruction),
                None => println!("{pc:08x}: {hex:<28} length-only"),
            },
        }
        pc = match pc.checked_add(len) {
            Some(next) => next,
            None => break,
        };
    }
}
