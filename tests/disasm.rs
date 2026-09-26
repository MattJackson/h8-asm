use h8_asm::{
    isa::{decode::decode, disasm::disassemble},
    Mode, Target,
};

fn rendered(bytes: [u8; 2]) -> String {
    let decoded = decode(&bytes, Target::H8_300, Mode::Normal).unwrap();
    disassemble(decoded.instruction)
}

#[test]
fn fixed_and_register_instructions() {
    for (bytes, expected) in [
        ([0x00, 0x00], "NOP"),
        ([0x01, 0x80], "SLEEP"),
        ([0x54, 0x70], "RTS"),
        ([0x56, 0x70], "RTE"),
        ([0x08, 0x19], "ADD.B R1H,R1L"),
        ([0x0c, 0xf8], "MOV.B R7L,R0L"),
    ] {
        assert_eq!(rendered(bytes), expected);
    }
}

#[test]
fn all_branch_conditions_and_signed_displacements() {
    let mnemonics = [
        "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
        "BLT", "BGT", "BLE",
    ];
    for (condition, mnemonic) in mnemonics.iter().enumerate() {
        assert_eq!(
            rendered([0x40 | condition as u8, 0]),
            format!("{mnemonic} $+2:8")
        );
        assert_eq!(
            rendered([0x40 | condition as u8, 0xfe]),
            format!("{mnemonic} $+0:8")
        );
    }
    assert_eq!(rendered([0x55, 0x80]), "BSR $-126:8");
    assert_eq!(rendered([0x55, 0x7e]), "BSR $+128:8");
}
