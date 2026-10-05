use super::StyleValues;
use crate::{
    ExtractStyleValue,
    vanilla_extract::{producer_atoms::ProducerAtoms, style_references::StyleReferences},
};

impl StyleValues {
    pub(crate) fn import_producer_atoms(&mut self, atoms: ProducerAtoms) {
        self.producer_atoms = atoms;
    }

    pub(crate) fn literal_parts(&self, literal: &str) -> Option<(Vec<ExtractStyleValue>, String)> {
        let tokens = self.producer_references.tokens(literal)?;
        let mut values = Vec::new();
        let mut residual = Vec::new();
        for (token, registered) in tokens {
            match self.producer_atoms.get(&token).filter(|_| registered) {
                Some(atoms) => values.extend_from_slice(atoms),
                None => residual.push(token),
            }
        }
        Some((values, residual.join(" ")))
    }

    pub(crate) fn has_literal_styles(&self, literal: &str) -> bool {
        self.producer_references.contains_class_list(literal)
    }

    pub(crate) fn import_producer_references(&mut self, references: StyleReferences) {
        self.producer_references = references;
    }
}
