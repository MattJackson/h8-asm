//! Semantic H8SX decoding and encoding driven by manual operand descriptors.
use super::{
    sx_table::{self, Row},
    sx_table_data::{FAMILIES, ROWS, SPECS},
    Decoded, Encoding, Insn,
};

fn candidate(row: &Row, mut insn: Insn) -> Option<u128> {
    insn.encoding = Encoding::Standard;
    let mut input = row.value;
    let spec = &SPECS[usize::from(row.index)];
    spec.write(&mut input, insn)?;
    if row.accepts(input) && spec.read(input) == insn {
        Some(input)
    } else {
        None
    }
}

fn canonical(insn: Insn, family: u16) -> Option<(u128, u8)> {
    FAMILIES[usize::from(family)]
        .rows
        .iter()
        .find_map(|&index| {
            let row = &ROWS[usize::from(index)];
            candidate(row, insn).map(|input| (input, row.len))
        })
}

pub(super) fn decode(bytes: &[u8]) -> Option<Decoded> {
    let row = sx_table::recognize(bytes)?;
    let mut input = [0u8; 16];
    let len = usize::from(row.len);
    input[..len].copy_from_slice(&bytes[..len]);
    let input = u128::from_be_bytes(input);
    let mut insn = SPECS[usize::from(row.index)].read(input);
    // Preserve distinct forms with equal operands without caching raw bytes.
    if canonical(insn, row.family) != Some((input, row.len)) {
        insn.encoding = Encoding::SxAlternative(row.index);
    }
    Some(Decoded { insn, len })
}

pub(super) fn encode(insn: Insn) -> Option<Vec<u8>> {
    let (input, len) = match insn.encoding {
        Encoding::Standard => {
            let family = FAMILIES
                .iter()
                .position(|family| family.mnemonic == insn.mnemonic && family.size == insn.size)?;
            canonical(insn, family as u16)?
        }
        Encoding::SxAlternative(index) => {
            let row = ROWS.get(usize::from(index))?;
            (candidate(row, insn)?, row.len)
        }
        Encoding::DisplacementStoreAlias => return None,
    };
    let bytes = input.to_be_bytes()[..usize::from(len)].to_vec();
    if decode(&bytes)
        == Some(Decoded {
            insn,
            len: usize::from(len),
        })
    {
        Some(bytes)
    } else {
        None
    }
}
