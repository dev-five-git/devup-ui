use super::{DevupVisitor, classify_attribute, value_of};
use crate::visit::order::{Item, Reach, Role, is_style, reach, suspends, written_properties};
use crate::visit::spread_slots::is_unknown_spread;
use css::disassemble_property;
use oxc_ast::ast::JSXAttributeItem::{self, Attribute, SpreadAttribute};
use oxc_ast::ast::JSXAttributeName::Identifier;
use rustc_hash::FxHashSet;
use std::borrow::Cow;

impl<'a> DevupVisitor<'a> {
    pub(super) fn attribute_items(
        &self,
        attributes: &[JSXAttributeItem<'a>],
        kept: &FxHashSet<String>,
    ) -> Vec<Item> {
        let mut written: FxHashSet<String> = FxHashSet::default();
        let mut lost = vec![false; attributes.len()];
        for (index, attribute) in attributes.iter().enumerate().rev() {
            let names: Vec<String> = match attribute {
                Attribute(attribute) => match &attribute.name {
                    Identifier(name) if is_style(&name.name, kept) => {
                        let names: Vec<_> = disassemble_property(&name.name)
                            .map(Cow::into_owned)
                            .collect();
                        lost[index] = names.len() > 1 || names.iter().all(|n| written.contains(n));
                        names
                    }
                    _ => Vec::new(),
                },
                SpreadAttribute(spread) => written_properties(&spread.argument)
                    .iter()
                    .flat_map(|key| disassemble_property(key).map(Cow::into_owned))
                    .collect(),
            };
            written.extend(names);
        }
        attributes
            .iter()
            .zip(lost)
            .map(|(attribute, lost)| match attribute {
                Attribute(attribute) => {
                    let value = value_of(attribute);
                    let (role, _) = classify_attribute(&attribute.name, kept);
                    Item {
                        role,
                        reach: value.map_or(Reach::Constant, |value| reach(&self.bindings, value)),
                        lost: lost || matches!(&attribute.name, Identifier(name) if name.name == "typography")
                            || (role == Role::Moved
                                && value.is_some_and(crate::visit::branch_evaluation::branches)),
                        snapshot: false,
                        suspends: value.is_some_and(suspends),
                    }
                }
                SpreadAttribute(spread) => {
                    let read = reach(&self.bindings, &spread.argument);
                    let unknown = is_unknown_spread(&spread.argument);
                    let snapshot = unknown;
                    Item {
                        role: if unknown { Role::Stays } else { Role::Moved },
                        reach: read,
                        lost: !unknown,
                        snapshot,
                        suspends: suspends(&spread.argument),
                    }
                }
            })
            .collect()
    }
}
