use h8_asm::{
    isa::{insn_len, sx_encode::encode, sx_semantic::decode},
    Mode, Target,
};

#[test]
fn every_two_byte_semantic_decode_agrees_with_length_and_encoder() {
    let mut decoded_words = 0;
    for word in 0..=u16::MAX {
        let bytes = word.to_be_bytes();
        if let Some(decoded) = decode(&bytes, Target::H8SX, Mode::Maximum) {
            assert_eq!(decoded.len, 2, "{word:04x}");
            assert_eq!(insn_len(&bytes, Target::H8SX, Mode::Maximum), Some(2));
            assert_eq!(
                encode(decoded.instruction, Target::H8SX, Mode::Maximum)
                    .unwrap()
                    .as_bytes(),
                bytes,
                "{word:04x}"
            );
            decoded_words += 1;
        }
    }
    assert_eq!(decoded_words, 39297);
}
