//! A label assembler for recognized H8 instruction bytes and relative branches.

use crate::{
    isa::insn_len,
    relocate::{self, Branch, Destination, Node, RelocateError},
    Mode, Target,
};
use std::collections::BTreeMap;

/// A label assembler refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmError {
    /// A label was defined more than once.
    DuplicateLabel(String),
    /// A referenced label has no definition.
    UndefinedLabel(String),
    /// A branch condition does not fit its four-bit field.
    InvalidCondition,
    /// Raw bytes must contain exactly one recognized, position-independent instruction.
    InvalidInstruction,
    /// CPU context, address or branch reach prevents assembly.
    Layout(RelocateError),
}

/// Builds code from exact instruction bytes and named branches.
///
/// Labels may be forward references. Finishing at an address performs branch
/// relaxation and returns a fresh byte vector. No global label state or I/O is
/// used. Byte instructions must not contain relative, delayed or PC-indexed
/// branches; express ordinary relative branches with [`Self::branch`] or
/// [`Self::call`].
pub struct Asm {
    target: Target,
    mode: Mode,
    nodes: Vec<Node>,
    labels: BTreeMap<String, usize>,
    references: Vec<(usize, String)>,
}

impl Asm {
    /// Creates an empty builder with an explicit CPU context.
    pub fn new(target: Target, mode: Mode) -> Result<Self, AsmError> {
        if !target.supports(mode) {
            return Err(AsmError::Layout(RelocateError::UnsupportedMode));
        }
        Ok(Self {
            target,
            mode,
            nodes: Vec::new(),
            labels: BTreeMap::new(),
            references: Vec::new(),
        })
    }

    /// Binds a label to the next instruction (or to the end of the block).
    /// Duplicate definitions leave the original binding intact.
    pub fn label(&mut self, name: &str) -> Result<(), AsmError> {
        if self.labels.contains_key(name) {
            return Err(AsmError::DuplicateLabel(name.into()));
        }
        self.labels.insert(name.into(), self.nodes.len());
        Ok(())
    }

    /// Appends one complete recognized position-independent instruction.
    pub fn instruction(&mut self, bytes: &[u8]) -> Result<(), AsmError> {
        if insn_len(bytes, self.target, self.mode) != Some(bytes.len()) {
            return Err(AsmError::InvalidInstruction);
        }
        if !matches!(relocate::classify(bytes, self.target, 0), Ok(None)) {
            return Err(AsmError::InvalidInstruction);
        }
        self.nodes.push(Node {
            bytes: bytes.to_vec(),
            branch: None,
        });
        Ok(())
    }

    /// Emits Bcc to a named label; condition 0 is unconditional BRA.
    pub fn branch(&mut self, condition: u8, label: &str) -> Result<(), AsmError> {
        if condition > 15 {
            return Err(AsmError::InvalidCondition);
        }
        self.reference(Branch::Conditional(condition), label);
        Ok(())
    }

    /// Emits BSR to a named label, widening if needed and supported by the CPU.
    pub fn call(&mut self, label: &str) {
        self.reference(Branch::Subroutine, label);
    }

    fn reference(&mut self, branch: Branch, label: &str) {
        self.references.push((self.nodes.len(), label.into()));
        self.nodes.push(Node {
            bytes: vec![0, 0],
            branch: Some((branch, Destination::Node(0))),
        });
    }

    /// Resolves labels and emits the block for the supplied address.
    pub fn finish(mut self, origin: u32) -> Result<Vec<u8>, AsmError> {
        for (index, label) in self.references {
            let destination = *self
                .labels
                .get(&label)
                .ok_or(AsmError::UndefinedLabel(label))?;
            self.nodes[index].branch.as_mut().unwrap().1 = Destination::Node(destination);
        }
        relocate::layout(self.nodes, origin, self.target, self.mode)
            .map(|(bytes, _)| bytes)
            .map_err(AsmError::Layout)
    }
}
