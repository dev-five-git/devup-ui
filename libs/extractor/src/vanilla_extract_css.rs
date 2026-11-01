use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsValue};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{StyleCollector, StyleEntry, array_items, js_str};

#[path = "vanilla_extract_css_values.rs"]
mod serialization;
use serialization::Values;

#[cfg(test)]
#[path = "vanilla_extract_css_key_slots_tests.rs"]
mod key_slots_tests;

fn unsupported() -> boa_engine::JsError {
    JsNativeError::typ()
        .with_message("css() needs static plain rules, CSS text or registered classes")
        .into()
}

pub(super) fn call(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let names = {
        let collector = collector.borrow();
        collector
            .styles
            .styles
            .keys()
            .chain(collector.styles.keyframes.keys())
            .map(|id| (id.clone(), id.clone()))
            .collect()
    };
    let mut values = Values {
        names: &names,
        collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: crate::own_css_plan::Plan {
            origin: collector.borrow().origin.clone(),
            ..crate::own_css_plan::Plan::default()
        },
    };
    let result = source(&mut values, args, context);
    let code = match result {
        Ok(code) => code,
        Err(error) => {
            let origin = collector.borrow().origin.clone();
            if let Some((location, api)) = origin {
                let message = format!(
                    "{location}: {}. Fix: use static plain rules, CSS text or registered classes",
                    crate::utils::build_time_error(
                        &api,
                        "<value>",
                        "its values must be static plain data"
                    )
                );
                collector.borrow_mut().declaration_errors.push(message);
            }
            return Err(error);
        }
    };
    let plan = values.plan.needs_bridge().then_some(values.plan);
    let code = plan
        .as_ref()
        .map_or(code, crate::own_css_plan::Plan::bounded_call);
    let mut collector = collector.borrow_mut();
    let id = collector.placeholder();
    collector.styles.styles.insert(
        id.clone(),
        StyleEntry {
            css: Some(code),
            own_plan: plan,
            json: "{}".into(),
            ..StyleEntry::default()
        },
    );
    Ok(boa_engine::js_string!(id).into())
}

fn source(values: &mut Values<'_>, args: &[JsValue], context: &mut Context) -> JsResult<String> {
    values.plan.composing = args.len() != 1
        || args
            .first()
            .and_then(JsValue::as_object)
            .is_some_and(|object| object.is_array());
    if let Some(first) = args
        .first()
        .and_then(JsValue::as_object)
        .filter(JsObject::is_array)
        && let Some(raw) =
            array_items(&first.get(boa_engine::js_string!("raw"), context)?, context)?
    {
        if let [_, value] = args
            && value.is_object()
            && raw
                .iter()
                .all(|part| js_str(part).is_some_and(|text| text.is_empty()))
        {
            values.plan.composing = value.as_object().is_some_and(|object| object.is_array());
            return Ok(format!("css({})", rule(values, value, context)?));
        }
        values.plan.template = Some(
            raw.iter()
                .map(|text| js_str(text).ok_or_else(unsupported))
                .collect::<JsResult<_>>()?,
        );
        let mut code = "css`".to_string();
        for (index, text) in raw.iter().enumerate() {
            code.push_str(&js_str(text).ok_or_else(unsupported)?);
            if let Some(value) = args.get(index + 1) {
                code.push_str("${");
                let root = if value.as_object().is_some_and(|object| object.is_array()) {
                    let start = values.plan.roots.len();
                    rule(values, value, context)?;
                    let items = values.plan.roots.drain(start..).collect();
                    let id = values.plan.nodes.len();
                    values
                        .plan
                        .nodes
                        .push(crate::own_css_plan::Node::Array(items));
                    id
                } else {
                    values.capture(value, context)?
                };
                match &values.plan.nodes[root] {
                    crate::own_css_plan::Node::Object(_) => values.declarations(root),
                    crate::own_css_plan::Node::Scalar(_) | crate::own_css_plan::Node::Array(_) => {}
                }
                values.plan.roots.push(root);
                if !values.plan.deep {
                    code.push_str(&values.plan.code(root));
                }
                code.push('}');
            }
        }
        code.push('`');
        return Ok(code);
    }
    let mut parts = Vec::new();
    for value in args {
        parts.push(rule(values, value, context)?);
    }
    Ok(format!("css({})", parts.join(",")))
}

fn rule(values: &mut Values<'_>, value: &JsValue, context: &mut Context) -> JsResult<String> {
    enum Step {
        Value(JsValue),
        Exit(JsObject),
    }
    let array = value.as_object().is_some_and(|object| object.is_array());
    let mut pending = vec![Step::Value(value.clone())];
    let mut parts = Vec::new();
    while let Some(step) = pending.pop() {
        match step {
            Step::Exit(object) => {
                values.active.remove(&object);
            }
            Step::Value(value) => {
                if let Some(items) = array_items(&value, context)? {
                    let object = value.as_object().ok_or_else(unsupported)?;
                    if !values.active.insert(object.clone()) {
                        return Err(unsupported());
                    }
                    pending.push(Step::Exit(object));
                    pending.extend(items.into_iter().rev().map(Step::Value));
                } else {
                    if value.is_number() {
                        return Err(unsupported());
                    }
                    let root = values.capture(&value, context)?;
                    if value.is_object() {
                        values.declarations(root);
                    }
                    values.plan.roots.push(root);
                    parts.push(if values.plan.deep {
                        String::new()
                    } else {
                        values.plan.code(root)
                    });
                }
            }
        }
    }
    let code = parts.join(",");
    Ok(if array { format!("[{code}]") } else { code })
}

pub(super) fn declarations(
    collected: &super::CollectedStyles,
    keyframes: &FxHashMap<String, String>,
    classes: &FxHashMap<&str, &str>,
) -> Option<Vec<String>> {
    if !collected.styles.values().any(|entry| entry.css.is_some()) {
        return None;
    }
    let mut references: Vec<_> = collected.references.iter().collect();
    references.sort_by_key(|(id, _)| super::placeholder_index(id));
    Some(
        references
            .into_iter()
            .filter_map(|(_, reference)| {
                let (name, entry, expression) = match reference {
                    super::Reference::Style { name, .. } => {
                        let entry = collected.styles.get(name)?;
                        (
                            name,
                            entry,
                            super::composed_css(collected, keyframes, classes, name, entry),
                        )
                    }
                    super::Reference::Keyframes(name) => {
                        let entry = collected.keyframes.get(name)?;
                        (
                            name,
                            entry,
                            format!(
                                "keyframes({})",
                                collected.resolve_json(&entry.json, keyframes)
                            ),
                        )
                    }
                };
                Some(format!(
                    "{}const {name} = {expression}",
                    super::export_prefix(entry.exported)
                ))
            })
            .collect(),
    )
}
