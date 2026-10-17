use super::super::super::{is_plain_object, string_code, to_text};
use super::super::{array_items, js_str, unsupported};
use super::Values;
use crate::own_css_plan::{Node, Plan, Property};
use boa_engine::{Context, JsObject, JsResult, JsValue, property::PropertyKey};

#[derive(Clone, Copy)]
enum Arrays {
    Data,
    Responsive,
    Entry,
}
enum Step {
    Value(JsValue, Arrays, usize, usize),
    Property(JsObject, PropertyKey, usize, usize),
    Exit(JsObject),
}

impl Values<'_> {
    pub(crate) fn capture(&mut self, value: &JsValue, context: &mut Context) -> JsResult<usize> {
        let root = self.plan.nodes.len();
        self.plan.nodes.push(Node::Array(Vec::new()));
        let mut pending = vec![Step::Value(value.clone(), Arrays::Data, root, 0)];
        while let Some(step) = pending.pop() {
            match step {
                Step::Exit(object) => {
                    self.active.remove(&object);
                }
                Step::Property(object, key, parent, depth) => {
                    let name = match &key {
                        PropertyKey::String(name) => name.to_std_string_escaped(),
                        PropertyKey::Index(index) => index.get().to_string(),
                        PropertyKey::Symbol(_) => return Err(unsupported()),
                    };
                    let slot = self.plan.slots.len();
                    self.plan.slots.push(string_code(&name, self.names));
                    let value = self.plan.nodes.len();
                    self.plan.nodes.push(Node::Array(Vec::new()));
                    let arrays = if data_slot(&name) {
                        Arrays::Data
                    } else {
                        Arrays::Responsive
                    };
                    if let Node::Object(properties) = &mut self.plan.nodes[parent] {
                        properties.push(Property { name, slot, value });
                    }
                    pending.push(Step::Value(object.get(key, context)?, arrays, value, depth));
                }
                Step::Value(value, arrays, id, depth) => {
                    // This bounds generated syntax, not accepted input depth; deeper values use the plan bridge.
                    self.plan.deep |= depth > 12;
                    let code = if let Some(text) = js_str(&value) {
                        Some(string_code(&text, self.names))
                    } else if value.is_null_or_undefined()
                        || value.as_boolean().is_some()
                        || value.as_number().is_some_and(f64::is_finite)
                    {
                        Some(to_text(&value, context)?)
                    } else {
                        None
                    };
                    if let Some(code) = code {
                        let slot = self.plan.slots.len();
                        self.plan.slots.push(code);
                        self.plan.nodes[id] = Node::Scalar(slot);
                        continue;
                    }
                    let object = value.as_object().ok_or_else(unsupported)?;
                    if object.is_callable() || !self.active.insert(object.clone()) {
                        return Err(unsupported());
                    }
                    pending.push(Step::Exit(object.clone()));
                    if let Some(items) = array_items(&value, context)? {
                        let children = match arrays {
                            Arrays::Data => Arrays::Data,
                            Arrays::Responsive => Arrays::Entry,
                            Arrays::Entry => {
                                return Err(boa_engine::JsNativeError::typ()
                                    .with_message(crate::utils::RESPONSIVE_ARRAY)
                                    .into());
                            }
                        };
                        let mut ids = Vec::with_capacity(items.len());
                        for _ in &items {
                            ids.push(self.plan.nodes.len());
                            self.plan.nodes.push(Node::Array(Vec::new()));
                        }
                        pending.extend(
                            items
                                .into_iter()
                                .zip(&ids)
                                .rev()
                                .map(|(item, &id)| Step::Value(item, children, id, depth + 1)),
                        );
                        self.plan.nodes[id] = Node::Array(ids);
                    } else if is_plain_object(&object, context) {
                        self.plan.nodes[id] = Node::Object(Vec::new());
                        pending.extend(
                            object
                                .own_property_keys(context)?
                                .into_iter()
                                .rev()
                                .map(|key| Step::Property(object.clone(), key, id, depth + 1)),
                        );
                    } else {
                        return Err(unsupported());
                    }
                }
            }
        }
        Ok(root)
    }
}

fn data_slot(name: &str) -> bool {
    matches!(name, "vars" | "props" | "styleVars" | "params" | "as")
        || css::is_special_property::is_special_property(name)
}

pub(super) fn validate(plan: &Plan, root: usize) -> Vec<(String, &'static str)> {
    use crate::dead_properties::ObjectKind;
    enum Step<'a> {
        Value(usize, ObjectKind, bool),
        Key(&'a Property, ObjectKind, bool),
        Push(String),
        Pop,
    }
    let mut pending = vec![Step::Value(root, ObjectKind::Styles, false)];
    let mut path = Vec::new();
    let mut errors = Vec::new();
    while let Some(step) = pending.pop() {
        match step {
            Step::Push(name) => path.push(name),
            Step::Pop => {
                path.pop();
            }
            Step::Key(property, kind, entry) => {
                path.push(property.name.clone());
                pending.push(Step::Pop);
                match kind {
                    ObjectKind::Styles => {
                        if let Some(requirement) =
                            crate::dead_properties::authored_requirement(&property.name)
                        {
                            errors.push((path.join(" -> "), requirement));
                        } else if !data_slot(&property.name) {
                            let kind = if matches!(
                                property.name.as_str(),
                                "selectors"
                                    | "@media"
                                    | "@supports"
                                    | "@container"
                                    | "_media"
                                    | "_supports"
                                    | "_container"
                                    | "@layer"
                            ) {
                                ObjectKind::Selectors
                            } else {
                                ObjectKind::Styles
                            };
                            pending.push(Step::Value(property.value, kind, entry));
                        }
                    }
                    ObjectKind::Selectors | ObjectKind::Records => {
                        pending.push(Step::Value(property.value, ObjectKind::Styles, entry));
                    }
                    ObjectKind::Globals => {}
                }
            }
            Step::Value(id, kind, entry) => match &plan.nodes[id] {
                Node::Scalar(_) => {}
                Node::Object(properties) => {
                    pending.extend(properties.iter().rev().map(|p| Step::Key(p, kind, entry)));
                }
                Node::Array(items) => {
                    if entry {
                        errors.push((path.join(" -> "), crate::utils::RESPONSIVE_ARRAY));
                        continue;
                    }
                    for (index, &item) in items.iter().enumerate().rev() {
                        pending.push(Step::Pop);
                        pending.push(Step::Value(item, kind, true));
                        pending.push(Step::Push(index.to_string()));
                    }
                }
            },
        }
    }
    errors
}
