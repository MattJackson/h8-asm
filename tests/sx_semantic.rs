use h8_asm::{
    isa::sx_semantic::{decode, SxInstruction},
    Mode, Target,
};

#[test]
fn ud04_dispatch_control_flow() {
    let mode = Mode::Maximum;
    let target = Target::H8SX;
    assert_eq!(
        decode(&[0xa8, 0xf0], target, mode).unwrap().instruction,
        SxInstruction::CompareByteImmediate {
            register: 8,
            immediate: 0xf0
        }
    );
    assert_eq!(
        decode(&[0x58, 0x70, 0x01, 0x14], target, mode)
            .unwrap()
            .instruction,
        SxInstruction::Branch16 {
            condition: 7,
            displacement: 0x114
        }
    );
    assert_eq!(
        decode(&[0x45, 0x06], target, mode).unwrap().instruction,
        SxInstruction::Branch8 {
            condition: 5,
            displacement: 6
        }
    );
    assert_eq!(
        decode(&[0x5e, 0x41, 0xce, 0xce], target, mode)
            .unwrap()
            .instruction,
        SxInstruction::Call24 { address: 0x41cece }
    );
    assert_eq!(
        decode(&[0x5a, 0x41, 0x94, 0xde], target, mode)
            .unwrap()
            .instruction,
        SxInstruction::Jump24 { address: 0x4194de }
    );
}

#[test]
fn ud04_absolute_byte_accesses() {
    let target = Target::H8SX;
    let mode = Mode::Maximum;
    assert_eq!(
        decode(&[0x6a, 0x08, 0x2c, 0x24], target, mode)
            .unwrap()
            .instruction,
        SxInstruction::LoadByteAbsolute16 {
            address: 0x2c24,
            register: 8,
        }
    );
    assert_eq!(
        decode(&[0x6a, 0x88, 0x2c, 0x24], target, mode)
            .unwrap()
            .instruction,
        SxInstruction::StoreByteAbsolute16 {
            address: 0x2c24,
            register: 8,
        }
    );
    assert!(decode(&[0x6a, 0x08, 0x2c], target, mode).is_none());
    assert!(decode(&[0x6a, 0x18, 0x2c, 0x24], target, mode).is_none());
    assert_eq!(
        decode(&[0xf8, 0xf1], target, mode).unwrap().instruction,
        SxInstruction::MoveByteImmediate {
            register: 8,
            immediate: 0xf1,
        }
    );
}

#[test]
fn strict_refusal_and_signed_displacements() {
    let t = Target::H8SX;
    let m = Mode::Maximum;
    assert_eq!(
        decode(&[0x47, 0xfe], t, m).unwrap().instruction,
        SxInstruction::Branch8 {
            condition: 7,
            displacement: -2
        }
    );
    assert_eq!(
        decode(&[0x58, 0x70, 0xff, 0xfc], t, m).unwrap().instruction,
        SxInstruction::Branch16 {
            condition: 7,
            displacement: -4
        }
    );
    assert!(decode(&[0x58, 0x71, 0, 0], t, m).is_none());
    assert!(decode(&[0x5e, 0x41], t, m).is_none());
    assert!(decode(&[0xa8, 0xf0], Target::H8_300, m).is_none());
}

#[test]
fn subroutine_returns_and_truncation() {
    let target = Target::H8SX;
    let mode = Mode::Normal;
    assert_eq!(
        decode(&[0x55, 0xfe], target, mode).unwrap().instruction,
        SxInstruction::BranchSubroutine8 { displacement: -2 }
    );
    assert_eq!(
        decode(&[0x5c, 0x00, 0xff, 0xfc], target, mode)
            .unwrap()
            .instruction,
        SxInstruction::BranchSubroutine16 { displacement: -4 }
    );
    assert_eq!(
        decode(&[0x54, 0x70], target, mode).unwrap().instruction,
        SxInstruction::Return
    );
    assert!(decode(&[], target, mode).is_none());
    assert!(decode(&[0x55], target, mode).is_none());
    assert!(decode(&[0x40, 0x01], target, mode).is_none());
    assert!(decode(&[0x55, 0x01], target, mode).is_none());
    assert!(decode(&[0x58, 0x00, 0x00, 0x01], target, mode).is_none());
    assert!(decode(&[0x5c, 0x00, 0x00, 0x01], target, mode).is_none());
    assert!(decode(&[0, 1], target, mode).is_none());
    for instruction in [
        [0x58, 0x70, 0x12, 0x34],
        [0x5c, 0x00, 0x12, 0x34],
        [0x5a, 0x41, 0x12, 0x34],
        [0x5e, 0x41, 0x12, 0x34],
    ] {
        for end in 0..4 {
            assert!(decode(&instruction[..end], target, mode).is_none());
        }
        assert_eq!(decode(&instruction, target, mode).unwrap().len, 4);
    }
}
