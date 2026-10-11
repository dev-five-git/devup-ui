use crate::ExtractStyleValue;
use crate::extract_style::style_property::StyleProperty;
use css::style_selector::StyleSelector;
use rustc_hash::{FxHashMap, FxHashSet};

/// Local declaration atoms indexed by the exact classes their producer emitted.
#[derive(Debug, Default, Clone)]
pub(crate) struct ProducerAtoms(FxHashMap<String, Vec<ExtractStyleValue>>);

impl ProducerAtoms {
    pub(crate) fn from_styles(styles: &FxHashSet<ExtractStyleValue>, scope: Option<&str>) -> Self {
        let mut atoms = Self::default();
        for value in styles {
            let local = match value {
                ExtractStyleValue::Static(style) => matches!(
                    style.selector(),
                    None | Some(StyleSelector::Selector(_) | StyleSelector::At { file: None, .. })
                ),
                ExtractStyleValue::Typography(_)
                | ExtractStyleValue::Dynamic(_)
                | ExtractStyleValue::Css(_)
                | ExtractStyleValue::Import(_)
                | ExtractStyleValue::FontFace(_)
                | ExtractStyleValue::Keyframes(_) => false,
            };
            if local && let Some(StyleProperty::ClassName(class)) = value.extract(scope) {
                let values = atoms.0.entry(class).or_default();
                if !values.contains(value) {
                    values.push(value.clone());
                }
            }
        }
        atoms
    }

    pub(crate) fn merge(&mut self, other: Self) {
        for (class, values) in other.0 {
            let existing = self.0.entry(class).or_default();
            for value in values {
                if !existing.contains(&value) {
                    existing.push(value);
                }
            }
        }
    }

    pub(crate) fn get(&self, class: &str) -> Option<&[ExtractStyleValue]> {
        self.0.get(class).map(Vec::as_slice)
    }

    pub(crate) fn alias(&mut self, alias: String, classes: &str) {
        let values = classes
            .split_whitespace()
            .filter_map(|class| self.get(class))
            .flatten()
            .cloned()
            .collect();
        self.0.insert(alias, values);
    }
}

#[cfg(test)]
mod tests;
