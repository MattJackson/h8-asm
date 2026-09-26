//! H8SX table matching after fixed-bit and operand-field validation.
use super::sx_table_data::{INDICES, OFFSETS, ROWS};

pub(super) struct Constraint(pub u8, pub u8, pub u8, pub u8);

pub(super) struct Row {
    pub mask: u128,
    pub value: u128,
    pub len: u8,
    pub name: &'static str,
    pub index: u16,
    pub family: u16,
    pub checks: [Constraint; 2],
}

impl Row {
    pub(super) fn accepts(&self, input: u128) -> bool {
        input & self.mask == self.value
            && self
                .checks
                .iter()
                .all(|&Constraint(shift, width, min, max)| {
                    let value = (input >> shift) & ((1u128 << width) - 1);
                    value >= u128::from(min) && value <= u128::from(max)
                })
    }
}

/// Match one complete instruction against the manually sourced row grammar.
pub(super) fn recognize(bytes: &[u8]) -> Option<&'static Row> {
    let first = usize::from(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]));
    let mut input = [0u8; 16];
    let count = bytes.len().min(14);
    input[..count].copy_from_slice(&bytes[..count]);
    let input = u128::from_be_bytes(input);
    let candidates = &INDICES[OFFSETS[first] as usize..OFFSETS[first + 1] as usize];
    for &index in candidates {
        let row = &ROWS[usize::from(index)];
        if usize::from(row.len) > bytes.len() {
            break;
        }
        if row.accepts(input) {
            return Some(row);
        }
    }
    None
}

pub(crate) fn complex_relative(bytes: &[u8]) -> bool {
    recognize(bytes).map_or(false, |row| {
        matches!(
            row.name,
            "BRA/BC" | "BRA/BS" | "BSR/BC" | "BSR/BS" | "MOVSD.B"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_manual_row_has_complete_and_truncated_boundary_checks() {
        for (index, row) in ROWS.iter().enumerate() {
            for ones in [false, true] {
                let mut input = row.value | if ones { !row.mask } else { 0 };
                for &Constraint(shift, width, min, max) in &row.checks {
                    let mask = ((1u128 << width) - 1) << shift;
                    input = (input & !mask) | (u128::from(if ones { max } else { min }) << shift);
                }
                let bytes = input.to_be_bytes();
                let bytes = &bytes[..usize::from(row.len)];
                let decoded = recognize(bytes);
                assert!(decoded.is_some(), "row {index} {}: {bytes:02x?}", row.name);
                let decoded = decoded.unwrap();
                assert_eq!(decoded.len, row.len, "row {index} {}", row.name);
                for end in 0..bytes.len() {
                    assert!(
                        recognize(&bytes[..end]).is_none(),
                        "row {index} prefix {end}"
                    );
                }
            }
        }
    }
}
