use h8_asm::{
    analysis::{function_start, reachable, xrefs, AnalysisError as E, XrefKind},
    relocate::RelocateError as R,
    Mode, Target,
};

#[test]
fn direct_references_use_mode_and_instruction_length() {
    let image = [
        0x46, 2, 0x55, 0xfc, 0x58, 0x70, 0xff, 0xf8, 0x5a, 0, 0, 14, 0x54, 0x70, 0x54, 0x70,
    ];
    let refs = xrefs(&image, 0..image.len(), Target::H8SX, Mode::Maximum).unwrap();
    assert_eq!(refs.len(), 4);
    assert_eq!(
        (refs[0].from, refs[0].to, refs[0].condition),
        (0, Some(4), Some(6))
    );
    assert_eq!((refs[1].kind, refs[1].to), (XrefKind::Call, Some(0)));
    assert_eq!(refs[2].to, Some(0));
    assert_eq!(refs[3].to, Some(14));
    assert_eq!(
        xrefs(&[0x40, 0xfc], 0..2, Target::H8SX, Mode::Normal).unwrap()[0].to,
        Some(65534)
    );
    let image = [0x5d, 8, 0x12, 0x34, 0x56, 0x78, 0x57, 0];
    let refs = xrefs(&image, 0..8, Target::H8SX, Mode::Maximum).unwrap();
    assert_eq!(refs[0].to, Some(0x12345678));
    assert_eq!(refs[0].kind, XrefKind::Call);
    assert_eq!(refs[1].to, None);
}

#[test]
fn vectors_are_big_endian_and_mode_sized() {
    let mut image = vec![0; 1024];
    image[..6].copy_from_slice(&[0x5b, 16, 0x5f, 20, 0x59, 0x80]);
    image[16..20].copy_from_slice(&[0, 0x40, 0, 0x60]);
    image[20..24].copy_from_slice(&[0xab, 0, 0, 0x80]);
    image[256..258].copy_from_slice(&[0, 0xa0]);
    image[512..516].copy_from_slice(&[0xab, 0, 0, 0xc0]);
    let normal = xrefs(&image, 0..6, Target::H8SX, Mode::Normal).unwrap();
    assert_eq!(normal[0].to, Some(0x40));
    assert_eq!(normal[0].vector, Some(16));
    assert_eq!(normal[1].kind, XrefKind::Call);
    assert_eq!(normal[2].to, Some(0xa0));
    assert_eq!(normal[2].vector, Some(256));
    let advanced = xrefs(&image, 0..6, Target::H8SX, Mode::Advanced).unwrap();
    assert_eq!(advanced[0].to, Some(0x400060));
    assert_eq!(advanced[1].to, Some(0x80));
    assert_eq!(advanced[2].to, Some(0xc0));
    assert_eq!(advanced[2].vector, Some(512));
    let maximum = xrefs(&image, 0..6, Target::H8SX, Mode::Maximum).unwrap();
    assert_eq!(maximum[1].to, Some(0xab000080));
    let missing = xrefs(&[0x5b, 255], 0..2, Target::H8SX, Mode::Normal).unwrap();
    assert_eq!(missing[0].to, None);
    let missing = xrefs(&[0x5d, 255], 0..2, Target::H8SX, Mode::Maximum).unwrap();
    assert_eq!(missing[0].to, None);
    let register = xrefs(&[0x59, 0, 0x5d, 0x17], 0..4, Target::H8SX, Mode::Normal).unwrap();
    assert!(register
        .iter()
        .all(|reference| reference.to.is_none() && reference.vector.is_none()));
}

#[test]
fn reachability_handles_loops_conditions_calls_and_returns() {
    // Conditional branch explores both paths; the call's target is not traversed.
    let image = [0x46, 4, 0x55, 0x40, 0x54, 0x70, 0x40, 0xfc];
    assert_eq!(
        reachable(&image, 0, Target::H8SX, Mode::Normal).unwrap(),
        [0, 2, 4, 6]
    );
    assert_eq!(
        reachable(&[0x40, 0xfe], 0, Target::H8_300, Mode::Normal).unwrap(),
        [0]
    );
    assert_eq!(
        reachable(&[0x41, 126, 0x54, 0x70], 0, Target::H8SX, Mode::Normal).unwrap(),
        [0, 2]
    );
    assert_eq!(
        reachable(&[0, 0, 0x54, 0x70], 0, Target::H8SX, Mode::Normal).unwrap(),
        [0, 2]
    );
    assert_eq!(
        reachable(&[0x5d, 0, 0x54, 0x70], 0, Target::H8SX, Mode::Normal).unwrap(),
        [0, 2]
    );
    let mut image = vec![0; 32];
    image[..2].copy_from_slice(&[0x5b, 16]);
    image[16..18].copy_from_slice(&[0, 24]);
    image[24..26].copy_from_slice(&[0x54, 0x70]);
    assert_eq!(
        reachable(&image, 0, Target::H8SX, Mode::Normal).unwrap(),
        [0, 24]
    );
}

#[test]
fn analysis_refusals_and_known_entry_function_lookup() {
    let t = Target::H8SX;
    let m = Mode::Normal;
    let image = [0, 0, 0x54, 0x70];
    assert_eq!(
        xrefs(&image, 0..4, Target::H8_300, Mode::Maximum),
        Err(E::Context(R::UnsupportedMode))
    );
    assert_eq!(xrefs(&image, 0..6, t, m), Err(E::OutOfBounds));
    assert_eq!(
        xrefs(&image, 1..3, t, m),
        Err(E::Context(R::InvalidAddress))
    );
    assert_eq!(xrefs(&image, 0..1, t, m), Err(E::UnsupportedInstruction(0)));
    assert_eq!(xrefs(&[0x40, 1], 0..2, t, m), Err(E::DelaySlot(0)));
    assert_eq!(
        xrefs(&[0x58, 0, 0, 1], 0..4, Target::H8_300H, Mode::Advanced),
        Err(E::UnsupportedInstruction(0))
    );
    assert_eq!(
        reachable(&image, 1, t, m),
        Err(E::Context(R::InvalidAddress))
    );
    assert_eq!(reachable(&image, 6, t, m), Err(E::OutOfBounds));
    assert_eq!(
        reachable(&[0, 1], 0, t, m),
        Err(E::UnsupportedInstruction(0))
    );
    assert_eq!(reachable(&[0x40, 1, 0, 0], 0, t, m), Err(E::DelaySlot(0)));
    assert_eq!(reachable(&[0x59, 0], 0, t, m), Err(E::UnresolvedBranch(0)));
    assert_eq!(
        reachable(&[0x46, 2, 0x79, 0, 0, 0, 0x54, 0x70], 0, t, m),
        Err(E::OverlappingInstructions(4))
    );
    // Visiting a long instruction first also refuses a later entry into its body.
    assert_eq!(
        reachable(&[0x46, 0, 0x79, 0, 0x40, 0, 0x40, 0xfc], 0, t, m),
        Err(E::OverlappingInstructions(4))
    );
    assert_eq!(function_start(&image, 2, &[0, 0], t, m).unwrap(), Some(0));
    assert_eq!(function_start(&image, 2, &[], t, m).unwrap(), None);
    assert_eq!(
        function_start(&image, 2, &[0, 2], t, m),
        Err(E::AmbiguousEntry(2))
    );
    assert_eq!(function_start(&image, 0, &[2], t, m).unwrap(), None);
    assert_eq!(
        function_start(&image, 1, &[], t, m),
        Err(E::Context(R::InvalidAddress))
    );
    assert_eq!(
        function_start(&[0x59, 0], 0, &[0], t, m),
        Err(E::UnresolvedBranch(0))
    );
}

#[test]
fn context_and_program_end_are_checked_in_each_public_walk() {
    assert_eq!(
        reachable(&[], 0, Target::H8_300, Mode::Maximum),
        Err(E::Context(R::UnsupportedMode))
    );
    assert_eq!(
        function_start(&[], 0, &[], Target::H8_300, Mode::Maximum),
        Err(E::Context(R::UnsupportedMode))
    );
    let mut image = vec![0; 65540];
    image[65534..65538].copy_from_slice(&[0x79, 0, 0, 0]);
    assert_eq!(
        reachable(&image, 65534, Target::H8SX, Mode::Normal),
        Err(E::Context(R::AddressOverflow))
    );
    assert_eq!(
        xrefs(&image, 65536..65538, Target::H8SX, Mode::Normal),
        Err(E::Context(R::InvalidAddress))
    );
    assert_eq!(
        xrefs(&image, 65534..65538, Target::H8SX, Mode::Normal),
        Err(E::Context(R::InvalidAddress))
    );
}

#[test]
fn bit_test_branch_does_not_fall_through_as_an_ordinary_instruction() {
    let bytes = [0x7c, 0, 0x40, 2];
    assert_eq!(
        h8_asm::analysis::reachable(&bytes, 0, Target::H8SX, Mode::Maximum),
        Err(h8_asm::analysis::AnalysisError::UnresolvedBranch(0))
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
        reachable(&bytes, 0, Target::H8SX, Mode::Maximum),
        Err(E::UnresolvedBranch(0))
    );
}

#[test]
fn pc_low_bit_is_ignored_for_direct_and_vector_targets() {
    for target in [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
        Target::H8SX,
    ] {
        for mode in [Mode::Normal, Mode::Middle, Mode::Advanced, Mode::Maximum] {
            if !target.supports(mode) {
                continue;
            }
            for op in [0x5a, 0x5e] {
                assert_eq!(
                    xrefs(&[op, 0, 0, 7], 0..4, target, mode).unwrap()[0].to,
                    Some(6)
                );
            }
            let mut image = vec![0; 32];
            image[..2].copy_from_slice(&[0x5b, 16]);
            if mode == Mode::Normal {
                image[17] = 7;
            } else {
                image[19] = 7;
            }
            assert_eq!(xrefs(&image, 0..2, target, mode).unwrap()[0].to, Some(6));
        }
    }
    assert_eq!(
        xrefs(&[0x59, 8, 0, 0, 0, 7], 0..6, Target::H8SX, Mode::Maximum).unwrap()[0].to,
        Some(6)
    );
}

#[test]
fn a_complete_instruction_can_end_exactly_at_the_normal_mode_limit() {
    let mut image = vec![0; 65536];
    image[65534..].copy_from_slice(&[0x54, 0x70]);
    assert_eq!(
        reachable(&image, 65534, Target::H8SX, Mode::Normal).unwrap(),
        [65534]
    );
    assert!(xrefs(&image, 65534..65536, Target::H8SX, Mode::Normal)
        .unwrap()
        .is_empty());
}

#[test]
fn vector_table_alignment_is_target_specific() {
    for target in [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
        Target::H8SX,
    ] {
        for mode in [Mode::Normal, Mode::Advanced] {
            if !target.supports(mode) {
                continue;
            }
            let mut image = vec![0; 32];
            image[..2].copy_from_slice(&[0x5b, 17]);
            if mode == Mode::Normal {
                image[17] = 7;
            } else {
                image[19] = 7;
            }
            let reference = xrefs(&image, 0..2, target, mode).unwrap()[0];
            if target == Target::H8SX {
                assert_eq!(reference.vector, Some(17));
                assert_eq!(reference.to, None);
                assert_eq!(
                    reachable(&image, 0, target, mode),
                    Err(E::UnresolvedBranch(0))
                );
            } else {
                assert_eq!(reference.vector, Some(16));
                assert_eq!(reference.to, Some(6));
            }
        }
    }
}
