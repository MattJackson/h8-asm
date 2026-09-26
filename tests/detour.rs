use h8_asm::{
    detour::{plan, tramp, DetourError as E},
    relocate::RelocateError as R,
    Mode, Target,
};

#[test]
fn mode_appropriate_hooks_and_padded_displaced_instructions() {
    for (target, mode, hook_len) in [
        (Target::H8_300, Mode::Normal, 4),
        (Target::H8_300H, Mode::Advanced, 4),
        (Target::H8S2600, Mode::Normal, 4),
        (Target::H8SX, Mode::Middle, 4),
        (Target::H8SX, Mode::Maximum, 6),
    ] {
        let mut image = vec![0xff; 128];
        image[..8].fill(0);
        let planned = plan(&image, 0, 16, 64, target, mode).unwrap();
        assert_eq!(planned.resume, hook_len);
        let installed = tramp(&mut image, 0, 16, 64, target, mode).unwrap();
        assert_eq!(planned, installed);
        assert_eq!(&image[..hook_len as usize], installed.site_bytes);
        assert_eq!(
            &image[64..64 + installed.trampoline_bytes.len()],
            installed.trampoline_bytes
        );
        if mode == Mode::Maximum {
            assert_eq!(installed.site_bytes, [0x59, 8, 0, 0, 0, 16]);
            assert_eq!(&installed.trampoline_bytes[6..], &[0x59, 8, 0, 0, 0, 6]);
        } else {
            assert_eq!(installed.site_bytes, [0x5a, 0, 0, 16]);
            assert_eq!(&installed.trampoline_bytes[4..], &[0x5a, 0, 0, 4]);
        }
    }
    let mut image = vec![0xff; 128];
    image[..6].copy_from_slice(&[0, 0, 0x79, 0, 0x12, 0x34]);
    let installed = tramp(&mut image, 0, 16, 64, Target::H8SX, Mode::Normal).unwrap();
    assert_eq!(installed.site_bytes, [0x5a, 0, 0, 16, 0, 0]);
    assert_eq!(installed.resume, 6);
    assert_eq!(
        &installed.trampoline_bytes[..6],
        &[0, 0, 0x79, 0, 0x12, 0x34]
    );
}

#[test]
fn every_refusal_is_atomic() {
    let mut original = vec![0xff; 128];
    original[..8].fill(0);
    for (site, hook, stub, target, mode, error) in [
        (
            0,
            16,
            64,
            Target::H8_300,
            Mode::Maximum,
            E::Relocation(R::UnsupportedMode),
        ),
        (
            1,
            16,
            64,
            Target::H8SX,
            Mode::Normal,
            E::Relocation(R::InvalidAddress),
        ),
        (
            0,
            17,
            64,
            Target::H8SX,
            Mode::Normal,
            E::Relocation(R::InvalidAddress),
        ),
        (
            0,
            16,
            65,
            Target::H8SX,
            Mode::Normal,
            E::Relocation(R::InvalidAddress),
        ),
        (128, 16, 64, Target::H8SX, Mode::Normal, E::OutOfBounds),
        (0, 128, 64, Target::H8SX, Mode::Normal, E::OutOfBounds),
        (0, 16, 128, Target::H8SX, Mode::Normal, E::OutOfBounds),
        (0, 16, 124, Target::H8SX, Mode::Normal, E::OutOfBounds),
        (0, 16, 2, Target::H8SX, Mode::Normal, E::Overlap),
        (0, 2, 64, Target::H8SX, Mode::Normal, E::Overlap),
        (0, 66, 64, Target::H8SX, Mode::Normal, E::Overlap),
        (0, 16, 6, Target::H8SX, Mode::Normal, E::NotFree),
    ] {
        let mut image = original.clone();
        assert_eq!(
            tramp(&mut image, site, hook, stub, target, mode),
            Err(error)
        );
        assert_eq!(image, original);
    }
    for (bytes, site, error) in [
        (vec![0x40, 1, 0, 0, 0, 0], 2, E::DelaySlot),
        (vec![0x40, 1, 0, 0], 0, E::Relocation(R::DynamicBranch(0))),
        (vec![0x59, 5, 0, 0], 0, E::Relocation(R::DynamicBranch(0))),
        (
            vec![0, 1, 0, 0],
            0,
            E::Relocation(R::UnsupportedInstruction(0)),
        ),
    ] {
        let mut image = original.clone();
        image[..bytes.len()].copy_from_slice(&bytes);
        let before = image.clone();
        assert_eq!(
            tramp(&mut image, site, 16, 64, Target::H8SX, Mode::Normal),
            Err(error)
        );
        assert_eq!(image, before);
    }
    let mut image = vec![0xff; 65536];
    image[65532..].fill(0);
    let before = image.clone();
    assert_eq!(
        tramp(&mut image, 65532, 16, 64, Target::H8SX, Mode::Normal),
        Err(E::Relocation(R::AddressOverflow))
    );
    assert_eq!(image, before);
}

#[test]
fn adjacent_ranges_and_exact_image_or_mode_end_are_allowed() {
    for (site, stub) in [(0usize, 4usize), (8, 0), (0, 24)] {
        let mut image = vec![0xff; 32];
        image[site..site + 4].fill(0);
        assert!(plan(
            &image,
            site as u32,
            16,
            stub as u32,
            Target::H8SX,
            Mode::Normal
        )
        .is_ok());
    }
    let mut image = vec![0xff; 65540];
    image[..4].fill(0);
    assert!(plan(&image, 0, 16, 65528, Target::H8SX, Mode::Normal).is_ok());
    assert_eq!(
        plan(&image, 0, 16, 65530, Target::H8SX, Mode::Normal),
        Err(E::OutOfBounds)
    );
    let mut image = vec![0xff; 32];
    image[..6].copy_from_slice(&[0x40, 0, 0, 0, 0, 0]);
    assert!(plan(&image, 2, 16, 24, Target::H8SX, Mode::Normal).is_ok());
}
