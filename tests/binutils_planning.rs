//! Independent GNU assembly of code produced by the label/detour planners.
use h8_asm::{detour, Asm, Mode, Target};
use std::{env, fs, process::Command};

#[test]
#[ignore = "requires H8SX-capable GNU as, ld and objcopy"]
fn planned_code_matches_independent_assembly() {
    let mut source = String::from(".h8300sx\n.text\n.global _start\n_start:\n");
    let mut expected = Vec::new();
    for mode in [Mode::Advanced, Mode::Maximum] {
        let mut image = vec![0xff; 128];
        image[..6].copy_from_slice(&[0, 0, 0x79, 0, 0x12, 0x34]);
        let plan = detour::plan(&image, 0, 16, 64, Target::H8SX, mode).unwrap();
        expected.extend_from_slice(&plan.site_bytes);
        expected.extend_from_slice(&plan.trampoline_bytes);
        if mode == Mode::Maximum {
            source.push_str("jmp @0x10:32\n");
        } else {
            source.push_str("jmp @0x10:24\nnop\n");
        }
        source.push_str("nop\nmov.w #0x1234:16,r0\n");
        if mode == Mode::Maximum {
            source.push_str("jmp @6:32\n");
        } else {
            source.push_str("jmp @6:24\n");
        }
    }
    let mut asm = Asm::new(Target::H8SX, Mode::Maximum).unwrap();
    asm.branch(0, "near").unwrap();
    asm.call("far");
    for _ in 0..62 {
        asm.instruction(&[0, 0]).unwrap();
    }
    asm.label("near").unwrap();
    for _ in 0..100 {
        asm.instruction(&[0, 0]).unwrap();
    }
    asm.label("far").unwrap();
    expected.extend(asm.finish(0).unwrap());
    source.push_str(
        "bra near:16\nbsr far:16\n.rept 62\nnop\n.endr\nnear:\n.rept 100\nnop\n.endr\nfar:\n",
    );
    let work = env::temp_dir().join(format!("h8-planning-oracle-{}", std::process::id()));
    fs::create_dir_all(&work).unwrap();
    fs::write(work.join("cases.s"), source).unwrap();
    for (variable, default, args) in [
        ("H8_AS", "h8300-elf-as", vec!["-o", "cases.o", "cases.s"]),
        (
            "H8_LD",
            "h8300-elf-ld",
            vec!["-m", "h8300sxelf", "-o", "cases.elf", "cases.o"],
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
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(fs::read(work.join("cases.bin")).unwrap(), expected);
    fs::remove_dir_all(work).unwrap();
}
