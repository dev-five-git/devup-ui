use super::producer_atoms::ProducerAtoms;
use super::{
    CollectedStyles, StyleEntry, array_items, inner_json, is_placeholder, js_str, json_string,
    style_to_json,
};
use boa_engine::{Context, JsResult, JsValue};
use rustc_hash::FxHashMap;

#[derive(Default)]
pub(crate) enum StyleOperandMode {
    #[default]
    Merged,
    Ordered,
    Generated(Option<String>),
}

#[derive(Debug, Clone)]
pub(crate) enum StyleOperand {
    Rules(String),
    Base(String),
    Classes(String),
}

pub(super) fn compose(
    value: &JsValue,
    entry: &mut StyleEntry,
    atoms: &ProducerAtoms,
    context: &mut Context,
) -> JsResult<()> {
    if let Some(items) = array_items(value, context)? {
        for item in &items {
            compose(item, entry, atoms, context)?;
        }
    } else if let Some(classes) = js_str(value) {
        let mut literal = Vec::new();
        for class in classes.split_whitespace() {
            if atoms.get(class).is_none() && is_placeholder(class) {
                if !literal.is_empty() {
                    entry
                        .operands
                        .push(StyleOperand::Classes(literal.join(" ")));
                    literal.clear();
                }
                entry.operands.push(StyleOperand::Base(class.to_string()));
            } else {
                literal.push(class);
            }
        }
        if !literal.is_empty() {
            entry
                .operands
                .push(StyleOperand::Classes(literal.join(" ")));
        }
    } else if value.is_object() {
        let json = style_to_json(value, context)?;
        if !inner_json(&json).is_empty() {
            entry.operands.push(StyleOperand::Rules(json));
        }
    }
    Ok(())
}

pub(super) struct Expansion<'a> {
    collected: &'a CollectedStyles,
    keyframes: &'a FxHashMap<String, String>,
    anchors: &'a FxHashMap<&'a str, &'a str>,
    composing: Vec<&'a str>,
    arguments: Vec<String>,
}

impl<'a> Expansion<'a> {
    pub(super) const fn new(
        collected: &'a CollectedStyles,
        keyframes: &'a FxHashMap<String, String>,
        anchors: &'a FxHashMap<&'a str, &'a str>,
    ) -> Self {
        Self {
            collected,
            keyframes,
            anchors,
            composing: Vec::new(),
            arguments: Vec::new(),
        }
    }

    fn expand(&mut self, name: &'a str, entry: &'a StyleEntry) {
        self.composing.push(name);
        for operand in &entry.operands {
            match operand {
                StyleOperand::Rules(json) => self
                    .arguments
                    .push(self.collected.resolve_json(json, self.keyframes)),
                StyleOperand::Classes(classes) => self.arguments.push(json_string(classes)),
                StyleOperand::Base(base) => {
                    if !self.composing.contains(&base.as_str())
                        && let Some(entry) = self.collected.styles.get(base)
                    {
                        self.expand(base, entry);
                    }
                }
            }
        }
        if let Some(anchor) = self.anchors.get(name) {
            self.arguments.push(json_string(anchor));
        }
        self.composing.pop();
    }

    pub(super) fn css(mut self, name: &'a str, entry: &'a StyleEntry) -> String {
        self.expand(name, entry);
        if self.arguments.is_empty() {
            self.arguments.push("{}".into());
        }
        format!("css({})", self.arguments.join(", "))
    }
}
