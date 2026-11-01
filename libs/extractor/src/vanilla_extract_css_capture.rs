use super::super::super::{is_plain_object, string_code, to_text};
use super::super::{js_str, unsupported};
use super::Values;
use crate::own_css_plan::{Node, Plan, Property};
use boa_engine::{Context, JsObject, JsResult, JsValue, property::PropertyKey};

#[cfg(test)]
#[path = "vanilla_extract_css_capture_tests.rs"]
mod tests;

thread_local! {
    // The evaluated module can replace Array.isArray; this realm never reaches it.
    static ARRAY_IS_ARRAY: JsResult<JsObject> = {
        let mut pristine = Context::default();
        pristine.intrinsics().constructors().array().constructor()
            .get(boa_engine::js_string!("isArray"), &mut pristine)
            .and_then(|value| value.as_callable().ok_or_else(unsupported))
    };
}

#[derive(Clone, Copy)]
enum Arrays {
    Data,
    Ignored,
    Params,
    ParamEntry,
    Responsive,
    Entry,
}
enum Step {
    Value(JsValue, Arrays, usize, usize),
    Property(JsObject, PropertyKey, usize, usize, bool),
    Exit(JsObject),
}

impl Values<'_> {
    pub(crate) fn capture(&mut self, value: &JsValue, context: &mut Context) -> JsResult<usize> {
        self.capture_with(value, context, Arrays::Data)
    }

    pub(crate) fn capture_data(
        &mut self,
        value: &JsValue,
        context: &mut Context,
    ) -> JsResult<usize> {
        self.capture_with(value, context, Arrays::Ignored)
    }

    fn capture_with(
        &mut self,
        value: &JsValue,
        context: &mut Context,
        arrays: Arrays,
    ) -> JsResult<usize> {
        let root = self.plan.nodes.len();
        self.plan.nodes.push(Node::Array(Vec::new()));
        let mut pending = vec![Step::Value(value.clone(), arrays, root, 0)];
        while let Some(step) = pending.pop() {
            match step {
                Step::Exit(object) => {
                    self.active.remove(&object);
                }
                Step::Property(object, key, parent, depth, metadata) => {
                    let name = match &key {
                        PropertyKey::String(name) => name.to_std_string_escaped(),
                        PropertyKey::Index(index) => index.get().to_string(),
                        PropertyKey::Symbol(_) => return Err(unsupported()),
                    };
                    let code = string_code(&name, self.names);
                    let slot = if let Some(&slot) = self.key_slots.get(&code) {
                        slot
                    } else {
                        let slot = self.plan.slots.len();
                        self.plan.slots.push(code.clone());
                        self.key_slots.insert(code, slot);
                        slot
                    };
                    let value = self.plan.nodes.len();
                    self.plan.nodes.push(Node::Array(Vec::new()));
                    let arrays = if !metadata {
                        Arrays::Ignored
                    } else if name == "params" {
                        self.plan.has_params = true;
                        Arrays::Params
                    } else if data_slot(&name) {
                        Arrays::Ignored
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
                    let code = if value.is_undefined() && matches!(arrays, Arrays::Params) {
                        Some("void 0".to_string())
                    } else if let Some(text) = js_str(&value) {
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
                        if matches!(arrays, Arrays::Params | Arrays::ParamEntry) {
                            use crate::own_css_lower::params::Scalar;
                            let scalar = if value.is_null_or_undefined() {
                                Scalar::Absent
                            } else {
                                Scalar::Text(js_str(&value).unwrap_or_else(|| code.clone()))
                            };
                            self.plan.param_values.insert(slot, scalar);
                        }
                        self.plan.slots.push(code);
                        self.plan.nodes[id] = Node::Scalar(slot);
                        continue;
                    }
                    if matches!(arrays, Arrays::ParamEntry) {
                        return Err(self.params_error());
                    }
                    let object = value.as_object().ok_or_else(|| {
                        if matches!(arrays, Arrays::Params) {
                            self.params_error()
                        } else {
                            unsupported()
                        }
                    })?;
                    let array = object.is_array()
                        || (!object.is_ordinary() && {
                            ARRAY_IS_ARRAY
                                .with(Clone::clone)?
                                .call(&JsValue::undefined(), std::slice::from_ref(&value), context)?
                                .to_boolean()
                        });
                    if matches!(arrays, Arrays::Params) && !array {
                        return Err(self.params_error());
                    }
                    if object.is_callable() || !self.active.insert(object.clone()) {
                        return Err(unsupported());
                    }
                    pending.push(Step::Exit(object.clone()));
                    if array {
                        let children = match arrays {
                            Arrays::Data => Arrays::Data,
                            Arrays::Ignored | Arrays::ParamEntry => Arrays::Ignored,
                            Arrays::Params => Arrays::ParamEntry,
                            Arrays::Responsive => Arrays::Entry,
                            Arrays::Entry => {
                                return Err(boa_engine::JsNativeError::typ()
                                    .with_message(crate::utils::RESPONSIVE_ARRAY)
                                    .into());
                            }
                        };
                        let length = object
                            .get(boa_engine::js_string!("length"), context)?
                            .to_u32(context)?;
                        let mut items = Vec::new();
                        let mut ids = Vec::new();
                        for index in 0..length {
                            let child = self.plan.nodes.len();
                            let item = object.get(index, context)?;
                            if matches!(arrays, Arrays::Params) {
                                // Refuse before queuing descendants or reading any later index.
                                if item.is_object() {
                                    return Err(self.params_error());
                                }
                                if item.is_undefined() && !object.has_property(index, context)? {
                                    self.plan.holes.insert(child);
                                }
                            }
                            items.push(item);
                            ids.push(child);
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
                        pending.extend(object.own_property_keys(context)?.into_iter().rev().map(
                            |key| {
                                Step::Property(
                                    object.clone(),
                                    key,
                                    id,
                                    depth + 1,
                                    !matches!(
                                        arrays,
                                        Arrays::Ignored | Arrays::Params | Arrays::ParamEntry
                                    ),
                                )
                            },
                        ));
                    } else {
                        return Err(unsupported());
                    }
                }
            }
        }
        Ok(root)
    }

    fn params_error(&self) -> boa_engine::JsError {
        if let Some((location, api)) = &self.plan.origin {
            self.collector.borrow_mut().declaration_errors.push(format!(
                "{location}: {}",
                crate::utils::build_time_error(
                    api,
                    "params",
                    crate::extractor::selector_params::SELECTOR_PARAMS
                )
            ));
        }
        unsupported()
    }
}

fn data_slot(name: &str) -> bool {
    matches!(name, "vars" | "props" | "styleVars" | "params" | "as")
        || css::is_special_property::is_special_property(name)
}

pub(super) fn validate(plan: &Plan, root: usize) -> Vec<(String, &'static str)> {
    #[derive(Clone, Copy)]
    enum ObjectKind {
        Styles,
        Selectors,
        At(css::style_selector::AtRuleKind),
        Layers,
    }
    enum Step<'a> {
        Value(usize, ObjectKind, bool, Option<usize>),
        Key(&'a Property, ObjectKind, bool, Option<usize>),
        Push(String),
        Pop,
    }
    let mut pending = vec![Step::Value(root, ObjectKind::Styles, false, None)];
    let mut contexts = crate::own_css_context::Contexts::default();
    let mut path = Vec::new();
    let mut errors = Vec::new();
    while let Some(step) = pending.pop() {
        match step {
            Step::Push(name) => path.push(name),
            Step::Pop => {
                path.pop();
            }
            Step::Key(property, kind, entry, mut context_path) => {
                path.push(property.name.clone());
                pending.push(Step::Pop);
                match kind {
                    ObjectKind::Styles => {
                        if let Some(requirement) =
                            crate::dead_properties::authored_requirement(&property.name)
                        {
                            errors.push((path.join(" -> "), requirement));
                        } else if !data_slot(&property.name) {
                            let name = property.name.as_str();
                            let kind = if name == "selectors" {
                                ObjectKind::Selectors
                            } else if name == "@layer" {
                                ObjectKind::Layers
                            } else if let Some(kind) =
                                crate::extractor::extract_style_from_expression::at_rule_record_kind(
                                    name,
                                )
                            {
                                ObjectKind::At(kind)
                            } else {
                                if plan.has_params
                                    && (name
                                        .starts_with([':', '.', '#', '[', '*', '>', '+', '~', '_'])
                                        || name.contains('&')
                                        || css::at_rule::split_at_rule_key(name).is_some())
                                    && let Some(frame) = crate::own_css_context::under(name)
                                {
                                    context_path = Some(contexts.push(context_path, frame));
                                }
                                ObjectKind::Styles
                            };
                            pending.push(Step::Value(property.value, kind, entry, context_path));
                        }
                    }
                    ObjectKind::Selectors | ObjectKind::At(_) | ObjectKind::Layers => {
                        let frame = match kind {
                            ObjectKind::Selectors => crate::own_css_context::under(&property.name),
                            ObjectKind::At(kind) => Some(crate::own_css_context::Frame::At(
                                kind,
                                property.name.clone(),
                            )),
                            ObjectKind::Layers => {
                                Some(crate::own_css_context::Frame::Layer(property.name.clone()))
                            }
                            ObjectKind::Styles => None,
                        };
                        if let Some(frame) = frame
                            && plan.has_params
                        {
                            context_path = Some(contexts.push(context_path, frame));
                        }
                        pending.push(Step::Value(
                            property.value,
                            ObjectKind::Styles,
                            entry,
                            context_path,
                        ));
                    }
                }
            }
            Step::Value(id, kind, entry, mut context_path) => match &plan.nodes[id] {
                Node::Scalar(_) => {}
                Node::Object(properties) => {
                    if matches!(kind, ObjectKind::Styles)
                        && contexts.selected(context_path)
                        && let Some(property) = properties.iter().find(|p| p.name == "params")
                    {
                        use crate::own_css_lower::params::{Params, Scalar};
                        match (Params {
                            plan,
                            value: property.value,
                        })
                        .resolve(
                            |slot| {
                                plan.param_values
                                    .get(&slot)
                                    .cloned()
                                    .unwrap_or(Scalar::Invalid)
                            },
                            || contexts.requires_argument(context_path),
                        ) {
                            Ok(Some(params)) => {
                                context_path = Some(contexts.push(
                                    context_path,
                                    crate::own_css_context::Frame::Params(params),
                                ));
                            }
                            Ok(None) => {}
                            Err(()) => {
                                errors.push((
                                    format!("{} -> params", path.join(" -> ")),
                                    crate::extractor::selector_params::SELECTOR_PARAMS,
                                ));
                                continue;
                            }
                        }
                    }
                    pending.extend(
                        properties
                            .iter()
                            .rev()
                            .map(|p| Step::Key(p, kind, entry, context_path)),
                    );
                }
                Node::Array(items) => {
                    if entry {
                        errors.push((path.join(" -> "), crate::utils::RESPONSIVE_ARRAY));
                        continue;
                    }
                    for (index, &item) in items.iter().enumerate().rev() {
                        pending.push(Step::Pop);
                        pending.push(Step::Value(item, kind, true, context_path));
                        pending.push(Step::Push(index.to_string()));
                    }
                }
            },
        }
    }
    errors
}
