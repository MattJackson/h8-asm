use h8_asm::{
    isa::{sx_disasm::disassemble, sx_semantic::decode},
    Mode, Target,
};

fn rendered(bytes: &[u8]) -> String {
    disassemble(
        decode(bytes, Target::H8SX, Mode::Maximum)
            .unwrap()
            .instruction,
    )
}

#[test]
fn compare_absolute_control_transfer_and_return() {
    for (bytes, expected) in [
        (&[0x6a, 0x08, 0x2c, 0x24][..], "MOV.B @H'2C24:16,R0L"),
        (&[0x6a, 0x88, 0x2c, 0x24][..], "MOV.B R0L,@H'2C24:16"),
        (&[0xf8, 0xf0][..], "MOV.B #H'F0,R0L"),
        (&[0xa8, 0xf0][..], "CMP.B #H'F0,R0L"),
        (&[0xa3, 0x01][..], "CMP.B #H'01,R3H"),
        (&[0x5a, 0x41, 0x94, 0xde][..], "JMP @H'4194DE:24"),
        (&[0x5e, 0x41, 0xce, 0xce][..], "JSR @H'41CECE:24"),
        (&[0x54, 0x70][..], "RTS"),
    ] {
        assert_eq!(rendered(bytes), expected);
    }
}

#[test]
fn branch_widths_and_pc_relative_offsets() {
    let names = [
        "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
        "BLT", "BGT", "BLE",
    ];
    for (condition, name) in names.iter().enumerate() {
        assert_eq!(
            rendered(&[0x40 | condition as u8, 0]),
            format!("{name} $+2:8")
        );
        assert_eq!(
            rendered(&[0x58, (condition as u8) << 4, 0, 0]),
            format!("{name} $+4:16")
        );
    }
    assert_eq!(rendered(&[0x47, 0xfe]), "BEQ $+0:8");
    assert_eq!(rendered(&[0x58, 0x70, 0xff, 0xfc]), "BEQ $+0:16");
    assert_eq!(rendered(&[0x55, 0x80]), "BSR $-126:8");
    assert_eq!(rendered(&[0x5c, 0x00, 0x7f, 0xfe]), "BSR $+32770:16");
}

#[test]
fn invalid_condition_has_no_valid_mnemonic() {
    use h8_asm::isa::sx_semantic::SxInstruction;
    assert_eq!(
        disassemble(SxInstruction::Branch8 {
            condition: 16,
            displacement: 0
        }),
        "B?? $+2:8"
    );
}
