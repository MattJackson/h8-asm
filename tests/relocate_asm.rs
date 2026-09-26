use h8_asm::{
    asm::AsmError,
    relocate::{relocate, RelocateError as E},
    Asm, Mode, Target,
};

#[test]
fn relocated_branches_keep_destinations_and_internal_boundaries() {
    let t = Target::H8SX;
    let m = Mode::Maximum;
    let moved = relocate(&[0x40, 2, 0, 0, 0, 0], 0x100, 0x1000, t, m).unwrap();
    assert_eq!(moved.bytes, [0x40, 2, 0, 0, 0, 0]);
    assert_eq!(moved.offsets, [(0, 0), (2, 2), (4, 4)]);
    // The same destination is external when it falls outside the source block.
    assert_eq!(
        relocate(&[0x40, 2], 0x100, 0x1000, t, m).unwrap().bytes,
        [0x58, 0, 0xf1, 0]
    );
    assert_eq!(
        relocate(&[0x55, 2], 0x100, 0x1000, t, m).unwrap().bytes,
        [0x5c, 0, 0xf1, 0]
    );
    assert_eq!(
        relocate(&[0x55, 2], 0x100, 0x102, t, m).unwrap().bytes,
        [0x55, 0]
    );
    assert_eq!(
        relocate(&[0x58, 0x70, 0, 2], 0x100, 0x102, t, m)
            .unwrap()
            .bytes,
        [0x58, 0x70, 0, 0]
    );
    assert_eq!(
        relocate(&[0x5c, 0, 0, 2], 0x100, 0x102, t, m)
            .unwrap()
            .bytes,
        [0x5c, 0, 0, 0]
    );
    // A branch to the old block end remains external.
    assert_eq!(
        relocate(&[0x40, 0], 0x100, 0x102, t, m).unwrap().bytes,
        [0x40, 0xfe]
    );
    assert_eq!(
        relocate(&[0x40, 0], 0, 0xfffc, t, Mode::Normal)
            .unwrap()
            .bytes,
        [0x40, 4]
    );
    assert_eq!(
        relocate(&[0x40, 0xfc], 0, 0xfffc, t, Mode::Normal)
            .unwrap()
            .bytes,
        [0x40, 0]
    );
    assert!(relocate(&[], 0, 0, t, m).unwrap().bytes.is_empty());
    for target in [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
        Target::H8SX,
    ] {
        for mode in [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum] {
            if target.supports(mode) {
                assert_eq!(relocate(&[0, 0], 2, 4, target, mode).unwrap().bytes, [0, 0]);
            }
        }
    }
}

#[test]
fn relocation_refuses_unsafe_or_unrepresentable_requests() {
    let t = Target::H8SX;
    let m = Mode::Maximum;
    assert_eq!(
        relocate(&[0, 0], 0, 0, Target::H8_300, m),
        Err(E::UnsupportedMode)
    );
    assert_eq!(relocate(&[0, 0], 1, 0, t, m), Err(E::InvalidAddress));
    assert_eq!(relocate(&[0, 0], 0, 1, t, m), Err(E::InvalidAddress));
    assert_eq!(
        relocate(&[0, 0], 0x10000, 0, t, Mode::Normal),
        Err(E::InvalidAddress)
    );
    assert_eq!(
        relocate(&[0, 0, 0, 0], 0xfffe, 0, t, Mode::Normal),
        Err(E::AddressOverflow)
    );
    assert_eq!(
        relocate(&[0, 0, 0, 0], 0, 0xfffe, t, Mode::Normal),
        Err(E::AddressOverflow)
    );
    assert_eq!(
        relocate(&[0], 0, 0, t, m),
        Err(E::UnsupportedInstruction(0))
    );
    assert_eq!(
        relocate(&[0x40, 1, 0, 0], 0, 0, t, m),
        Err(E::DynamicBranch(0))
    );
    assert_eq!(relocate(&[0x59, 5], 0, 0, t, m), Err(E::DynamicBranch(0)));
    assert_eq!(
        relocate(&[0x5d, 0x77], 0, 0, t, m),
        Err(E::DynamicBranch(0))
    );
    assert_eq!(
        relocate(&[0x58, 0, 0, 1], 0, 0, Target::H8_300H, Mode::Advanced),
        Err(E::UnsupportedInstruction(0))
    );
    assert_eq!(
        relocate(&[0x40, 2, 0x79, 0, 0, 0], 0, 0, t, m),
        Err(E::InteriorBranch(4))
    );
    assert_eq!(
        relocate(&[0x40, 0], 0, 0x8000, Target::H8_300, Mode::Normal),
        Err(E::BranchOutOfRange)
    );
    assert_eq!(
        relocate(&[0x40, 0], 0, 0x10000, t, m),
        Err(E::BranchOutOfRange)
    );
}

#[test]
fn assembler_resolves_forward_backward_and_end_labels() {
    let mut asm = Asm::new(Target::H8SX, Mode::Maximum).unwrap();
    asm.label("start").unwrap();
    asm.branch(7, "end").unwrap();
    asm.instruction(&[0, 0]).unwrap();
    asm.call("start");
    asm.label("end").unwrap();
    assert_eq!(asm.finish(0x100).unwrap(), [0x47, 4, 0, 0, 0x55, 0xfa]);
    let mut asm = Asm::new(Target::H8SX, Mode::Normal).unwrap();
    asm.branch(0, "end").unwrap();
    asm.label("end").unwrap();
    assert_eq!(asm.finish(0xfffe).unwrap(), [0x40, 0]);
}

#[test]
fn assembler_repeats_relaxation_when_one_widening_forces_another() {
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
    let bytes = asm.finish(0).unwrap();
    assert_eq!(&bytes[..8], &[0x58, 0, 0, 128, 0x5c, 0, 1, 68]);
    assert_eq!(bytes.len(), 332);
}

#[test]
fn builder_refusals_do_not_mutate_its_existing_content() {
    assert!(matches!(
        Asm::new(Target::H8_300, Mode::Maximum),
        Err(AsmError::Layout(E::UnsupportedMode))
    ));
    let mut asm = Asm::new(Target::H8_300, Mode::Normal).unwrap();
    asm.label("x").unwrap();
    asm.instruction(&[0, 0]).unwrap();
    assert_eq!(asm.label("x"), Err(AsmError::DuplicateLabel("x".into())));
    assert_eq!(asm.branch(16, "x"), Err(AsmError::InvalidCondition));
    for bytes in [&[][..], &[0], &[0, 0, 0, 0], &[0x40, 0]] {
        assert_eq!(asm.instruction(bytes), Err(AsmError::InvalidInstruction));
    }
    asm.branch(0, "x").unwrap();
    assert_eq!(asm.finish(0).unwrap(), [0, 0, 0x40, 0xfc]);
    let mut asm = Asm::new(Target::H8SX, Mode::Maximum).unwrap();
    assert_eq!(
        asm.instruction(&[0x40, 1]),
        Err(AsmError::InvalidInstruction)
    );
    asm.call("missing");
    assert_eq!(
        asm.finish(0),
        Err(AsmError::UndefinedLabel("missing".into()))
    );
    assert_eq!(
        Asm::new(Target::H8SX, Mode::Maximum).unwrap().finish(1),
        Err(AsmError::Layout(E::InvalidAddress))
    );
}

#[test]
fn earlier_widening_can_bring_an_external_long_branch_into_range() {
    let bytes = [0x40, 0xfc, 0x58, 0, 0, 0];
    assert_eq!(
        relocate(&bytes, 0x10000, 0x8000, Target::H8SX, Mode::Maximum)
            .unwrap()
            .bytes,
        [0x58, 0, 0x7f, 0xfa, 0x58, 0, 0x7f, 0xfe]
    );
}

#[test]
fn absolute_internal_transfers_are_refused_and_external_ones_preserved() {
    for bytes in [
        vec![0x5a, 0, 1, 0],
        vec![0x5e, 0, 1, 0],
        vec![0x59, 8, 0, 0, 1, 0],
        vec![0x5d, 8, 0, 0, 1, 0],
    ] {
        assert_eq!(
            relocate(&bytes, 0x100, 0x200, Target::H8SX, Mode::Maximum),
            Err(E::AbsoluteInternalBranch(0x100))
        );
        assert_eq!(
            relocate(&bytes, 0x200, 0x300, Target::H8SX, Mode::Maximum)
                .unwrap()
                .bytes,
            bytes
        );
    }
    let bytes = [0x5a, 1, 1, 0];
    assert_eq!(
        relocate(&bytes, 0x100, 0x200, Target::H8SX, Mode::Normal),
        Err(E::AbsoluteInternalBranch(0x100))
    );
}

#[test]
fn newly_recognized_bit_test_branches_are_not_copied_unchanged() {
    let bytes = [0x7c, 0, 0x40, 2];
    assert_eq!(
        h8_asm::isa::insn_len(&bytes, Target::H8SX, Mode::Maximum),
        Some(4)
    );
    assert_eq!(
        h8_asm::relocate::relocate(&bytes, 0, 0x100, Target::H8SX, Mode::Maximum),
        Err(h8_asm::relocate::RelocateError::DynamicBranch(0))
    );
}

#[test]
fn movsd_relative_target_is_not_silently_copied_or_treated_as_fallthrough() {
    let bytes = [0x7b, 0x84, 0, 2];
    assert_eq!(
        h8_asm::isa::insn_len(&bytes, Target::H8SX, Mode::Maximum),
        Some(4)
    );
    assert_eq!(
        relocate(&bytes, 0, 0x100, Target::H8SX, Mode::Maximum),
        Err(E::DynamicBranch(0))
    );
}

#[test]
fn odd_absolute_pc_value_still_targets_the_overwritten_instruction() {
    for bytes in [vec![0x5a, 0, 1, 1], vec![0x59, 8, 0, 0, 1, 1]] {
        assert_eq!(
            relocate(&bytes, 0x100, 0x200, Target::H8SX, Mode::Maximum),
            Err(E::AbsoluteInternalBranch(0x100))
        );
    }
}

#[test]
fn label_builder_accepts_the_last_condition_code() {
    let mut asm = Asm::new(Target::H8_300, Mode::Normal).unwrap();
    asm.branch(15, "done").unwrap();
    asm.label("done").unwrap();
    assert_eq!(asm.finish(0).unwrap(), [0x4f, 0]);
}

#[test]
fn source_end_is_exclusive_and_may_equal_the_address_space_end() {
    assert_eq!(
        relocate(&[0, 0], 65534, 0, Target::H8_300, Mode::Normal)
            .unwrap()
            .bytes,
        [0, 0]
    );
    let bytes = [0x5a, 0, 1, 4];
    assert_eq!(
        relocate(&bytes, 0x100, 0x200, Target::H8SX, Mode::Maximum)
            .unwrap()
            .bytes,
        bytes
    );
}
