use crate::{
    ExtractStyleProp,
    extractor::{
        ExtractResult,
        extract_style_from_expression::{
            LiteralHandling, at_rule_record_kind, extract_style_from_expression, misplaced,
            place_in_layer,
        },
    },
    own_css_context::{Contexts, Frame, under},
    own_css_plan::{Node, Plan},
    utils::{RESPONSIVE_ARRAY, SELECTOR_NAME, get_string_by_literal_expression},
};
use css::{disassemble_property, style_selector::split_selector_list};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};

struct Cursor {
    id: usize,
    name: Option<String>,
    level: u8,
    single: bool,
    path: Option<usize>,
}
enum Step {
    Value(Cursor),
    Array(usize),
}

pub(crate) struct RuleCompiler<'r, 'a> {
    pub ast: &'r AstBuilder<'a>,
    pub plan: &'r Plan,
    pub slots: &'r [Expression<'a>],
}

impl<'a> RuleCompiler<'_, 'a> {
    pub fn lower(&self, root: usize) -> ExtractResult<'a> {
        let Self { ast, plan, slots } = *self;
        let mut pending = vec![Step::Value(Cursor {
            id: root,
            name: None,
            level: 0,
            single: false,
            path: None,
        })];
        let mut contexts = Contexts::default();
        let mut result = ExtractResult::default();
        while let Some(step) = pending.pop() {
            let cursor = match step {
                Step::Value(cursor) => cursor,
                Step::Array(start) => {
                    let styles = result.styles.split_off(start);
                    result.styles.push(ExtractStyleProp::StaticArray(styles));
                    continue;
                }
            };
            let Cursor {
                id,
                name,
                level,
                single,
                mut path,
            } = cursor;
            let node = &plan.nodes[id];
            if let Some(name) = name.as_deref() {
                let object = match node {
                    Node::Object(properties) => Some(properties),
                    Node::Scalar(_) | Node::Array(_) => None,
                };
                if name == "as" {
                    continue;
                }
                if let Some(properties) = object {
                    let record = at_rule_record_kind(name);
                    if name == "selectors" || name == "@layer" || record.is_some() {
                        for property in properties.iter().rev() {
                            let keys = if name == "selectors" {
                                split_selector_list(property.name.trim())
                            } else {
                                vec![property.name.as_str()]
                            };
                            for key in keys.into_iter().rev() {
                                let frame = if name == "@layer" {
                                    Some(Frame::Layer(key.to_string()))
                                } else if let Some(kind) = record {
                                    Some(Frame::At(kind, key.to_string()))
                                } else {
                                    under(key)
                                };
                                if let Some(frame) = frame {
                                    let path = Some(contexts.push(path, frame));
                                    pending.push(Step::Value(Cursor {
                                        id: property.value,
                                        name: None,
                                        level,
                                        single,
                                        path,
                                    }));
                                } else {
                                    result.styles.push(misplaced(
                                        0,
                                        key.to_string(),
                                        SELECTOR_NAME,
                                    ));
                                }
                            }
                        }
                        continue;
                    }
                }
                if name.starts_with([':', '.', '#', '[', '*', '>', '+', '~', '_'])
                    || name.contains('&')
                    || css::at_rule::split_at_rule_key(name).is_some()
                {
                    let keys = split_selector_list(name);
                    for key in keys.into_iter().rev() {
                        if let Some(frame) = under(key) {
                            pending.push(Step::Value(Cursor {
                                id,
                                name: None,
                                level,
                                single,
                                path: Some(contexts.push(path, frame)),
                            }));
                        } else {
                            result
                                .styles
                                .push(misplaced(0, key.to_string(), SELECTOR_NAME));
                        }
                    }
                    continue;
                }
            }
            match node {
                Node::Object(properties) => {
                    let root_object = name.is_none() && !contexts.selected(path);
                    if !root_object
                        && contexts.selected(path)
                        && let Some(property) = properties.iter().find(|p| p.name == "params")
                        && let Node::Array(items) = &plan.nodes[property.value]
                    {
                        let params: Vec<_> = items
                            .iter()
                            .filter_map(|id| match plan.nodes[*id] {
                                Node::Scalar(slot) => {
                                    get_string_by_literal_expression(&slots[slot])
                                        .filter(|s| !s.is_empty())
                                }
                                Node::Array(_) | Node::Object(_) => None,
                            })
                            .collect();
                        path = Some(contexts.push(path, Frame::Params(params.join(","))));
                    }
                    let properties: Vec<_> = if root_object {
                        properties.iter().collect()
                    } else {
                        properties.iter().rev().collect()
                    };
                    for property in properties {
                        if root_object {
                            if property.name == "styleOrder" {
                                if id == root
                                    && let Node::Scalar(slot) = plan.nodes[property.value]
                                {
                                    result.style_order =
                                        crate::utils::get_number_by_literal_expression(
                                            &slots[slot],
                                        )
                                        .and_then(|n| {
                                            n.clamp(0.0, 255.0).trunc().to_string().parse().ok()
                                        });
                                }
                                continue;
                            }
                            if matches!(property.name.as_str(), "props" | "styleVars")
                                || css::is_special_property::is_special_property(&property.name)
                            {
                                continue;
                            }
                        } else if property.name == "params" {
                            continue;
                        }
                        let start = pending.len();
                        for name in disassemble_property(&property.name) {
                            pending.push(Step::Value(Cursor {
                                id: property.value,
                                name: Some(name.into_owned()),
                                level: if root_object { 0 } else { level },
                                single: if root_object { false } else { single },
                                path,
                            }));
                        }
                        pending[start..].reverse();
                    }
                }
                Node::Array(items) => {
                    if single {
                        result
                            .styles
                            .push(misplaced(0, "<array>".into(), RESPONSIVE_ARRAY));
                        continue;
                    }
                    pending.push(Step::Array(result.styles.len()));
                    for (index, &id) in items.iter().enumerate().rev() {
                        let level = index.to_le_bytes()[0];
                        pending.push(Step::Value(Cursor {
                            id,
                            name: name.clone(),
                            level,
                            single: true,
                            path,
                        }));
                    }
                }
                Node::Scalar(slot) => {
                    let Some((selector, layer)) = contexts.materialize(path) else {
                        continue;
                    };
                    let mut expression = slots[*slot].clone_in_with_semantic_ids(ast.allocator());
                    let mut styles = extract_style_from_expression(
                        ast,
                        name.as_deref(),
                        &mut expression,
                        level,
                        &selector,
                        if single {
                            LiteralHandling::KeepSingleClass
                        } else {
                            LiteralHandling::ExpandResponsiveThemeToken
                        },
                    )
                    .styles;
                    if let Some(layer) = layer {
                        place_in_layer(&mut styles, &layer);
                    }
                    result.styles.extend(styles);
                }
            }
        }
        result
    }
}
