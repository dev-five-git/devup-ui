use crate::{
    ExtractStyleProp,
    extract_style::extract_style_value::ExtractStyleValue,
    extractor::{
        GlobalExtractResult,
        extract_keyframes_from_expression::extract_keyframes_with_location,
        extract_style_from_expression::{
            LiteralHandling, at_rule_record_kind, extract_style_from_expression, misplaced,
            place_in_layer, unreadable, unreadable_key, yield_typography,
        },
        global_inputs::{GlobalInputs, font_faces, imports},
    },
    utils::{
        SELECTOR_NAME, get_str_by_property_key, get_string_by_literal_expression,
        get_string_by_property_key, unwrap_syntax_only_mut,
    },
};
use css::{
    at_rule::{media_shorthand_query, split_at_rule_key},
    style_selector::{AtRule, AtRuleKind, StyleSelector, is_selector_name},
    utils::to_kebab_case,
};
use oxc_ast::{
    ast::{Expression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::GetSpan;

pub fn extract_global_style_from_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    inputs: &GlobalInputs<'_, 'a>,
) -> GlobalExtractResult<'a> {
    let invalid: Vec<_> = crate::dead_properties::authored_errors(
        expression,
        crate::dead_properties::ObjectKind::Globals,
    )
    .into_iter()
    .map(|(offset, code, requirement)| ExtractStyleProp::Unreadable {
        offset,
        code,
        prop: false,
        requirement: Some(requirement),
    })
    .collect();
    if !invalid.is_empty() {
        return GlobalExtractResult {
            styles: invalid,
            style_order: None,
        };
    }
    let mut styles = vec![];
    collect_global_styles(ast_builder, expression, inputs, &[], &mut styles);
    GlobalExtractResult {
        styles,
        style_order: None,
    }
}

/// A top-level at-rule key wrapping a selector map: `'@media print'` or a
/// media shorthand such as `_print` / `_motionReduce`.
fn global_at_rule_key(name: &str) -> Option<AtRule> {
    let (kind, query) = split_at_rule_key(name).or_else(|| {
        let query = media_shorthand_query(name.strip_prefix('_')?)?;
        Some((AtRuleKind::Media, query))
    })?;
    Some(AtRule {
        kind,
        query: query.to_string(),
    })
}

/// What a `'@keyframes name'` key requires
const KEYFRAMES_KEY: &str = "a `@keyframes` key must name its keyframes with letters, digits, `-` and `_`, outside any at-rule";

/// The name a `'@keyframes name'` key declares
fn keyframes_name(key: &str) -> Option<&str> {
    let name = key.strip_prefix("@keyframes")?;
    name.starts_with(char::is_whitespace).then(|| name.trim())
}

/// Whether `name` is a keyframes name CSS takes unquoted
fn is_keyframes_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_') || !c.is_ascii())
}

/// `'@keyframes name': { from: {...}, to: {...} }`: keyframes declared under
/// `name` as written, as vanilla-extract's `globalKeyframes` declares them
fn global_keyframes<'a>(
    ast_builder: &AstBuilder<'a>,
    (name, offset): (&str, u32),
    value: &mut Expression<'a>,
    inputs: &GlobalInputs<'_, 'a>,
    nested: bool,
) -> ExtractStyleProp<'a> {
    if nested || !is_keyframes_name(name) {
        return misplaced(offset, format!("'@keyframes {name}'"), KEYFRAMES_KEY);
    }
    let (result, runtime_offset) = extract_keyframes_with_location(
        super::style_inputs::StyleInputs::with_values(ast_builder, inputs.values),
        value,
    );
    if let Some((code, requirement)) = result.runtime_value {
        return ExtractStyleProp::Unreadable {
            offset: runtime_offset.unwrap_or_else(|| value.span().start),
            code,
            prop: false,
            requirement,
        };
    }
    let mut keyframes = result.keyframes;
    keyframes.name = Some(name.to_string());
    ExtractStyleProp::Static(ExtractStyleValue::Keyframes(keyframes))
}

/// Collect a `globalCss` selector map whose rules sit inside `at_rules`
/// (outermost first).
fn collect_global_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    inputs: &GlobalInputs<'_, 'a>,
    at_rules: &[AtRule],
    styles: &mut Vec<ExtractStyleProp<'a>>,
) {
    let expression = unwrap_syntax_only_mut(expression);
    let invalid = crate::dead_properties::expression_errors(expression);
    if !invalid.is_empty() {
        styles.extend(invalid);
        return;
    }

    if let Expression::ObjectExpression(obj) = expression {
        for p in &mut obj.properties {
            match p {
                ObjectPropertyKind::ObjectProperty(o) => {
                    if let Some(name) = get_string_by_property_key(&o.key) {
                        if let Some(kind) = at_rule_record_kind(&name)
                            && let Expression::ObjectExpression(record) = &mut o.value
                        {
                            for entry in &mut record.properties {
                                if let ObjectPropertyKind::ObjectProperty(entry) = entry
                                    && let Some(query) = get_string_by_property_key(&entry.key)
                                {
                                    let mut nested = at_rules.to_vec();
                                    nested.push(AtRule { kind, query });
                                    collect_global_styles(
                                        ast_builder,
                                        &mut entry.value,
                                        inputs,
                                        &nested,
                                        styles,
                                    );
                                }
                            }
                        } else if let Some(layer) = name
                            .strip_prefix("@layer")
                            .filter(|rest| rest.starts_with(char::is_whitespace))
                            .map(str::trim)
                        {
                            if layer.is_empty()
                                || !layer.chars().all(|c| {
                                    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
                                })
                            {
                                styles.push(unreadable_key(&o.key, false));
                                continue;
                            }
                            // `'@layer name': { selector: rules }` puts the rules it holds in the layer
                            let mut layered = vec![];
                            collect_global_styles(
                                ast_builder,
                                &mut o.value,
                                inputs,
                                at_rules,
                                &mut layered,
                            );
                            place_in_layer(&mut layered, layer);
                            styles.extend(layered);
                        } else if let Some(frames) = keyframes_name(&name) {
                            styles.push(global_keyframes(
                                ast_builder,
                                (frames, o.key.span().start),
                                &mut o.value,
                                inputs,
                                !at_rules.is_empty(),
                            ));
                        } else if let Some(at_rule) = global_at_rule_key(&name) {
                            let mut nested = at_rules.to_vec();
                            nested.push(at_rule);
                            collect_global_styles(
                                ast_builder,
                                &mut o.value,
                                inputs,
                                &nested,
                                styles,
                            );
                        } else if name == "imports" {
                            styles.extend(imports(ast_builder, &mut o.value, inputs));
                        } else if name == "fontFaces" {
                            styles.extend(font_faces(ast_builder, &mut o.value, inputs));
                        } else {
                            // Handle @layer property in globalStyle
                            // Extract the layer name if present in the style object
                            let layer_name = if let Expression::ObjectExpression(style_obj) = &o.value
                                && let Some(ObjectPropertyKind::ObjectProperty(sp)) = style_obj.properties.iter().find(|style_prop| matches!(style_prop, ObjectPropertyKind::ObjectProperty(s) if get_str_by_property_key(&s.key).as_deref() == Some("@layer")))
                            {
                                get_string_by_literal_expression(&sp.value)
                            } else {
                                None
                            };

                            let global = StyleSelector::Global(
                                if let Some(pseudo) = name.strip_prefix('_') {
                                    let pseudo = to_kebab_case(pseudo);
                                    if !is_selector_name(&pseudo) {
                                        styles.push(misplaced(
                                            o.key.span().start,
                                            name,
                                            SELECTOR_NAME,
                                        ));
                                        continue;
                                    }
                                    css::quoted::replace_unquoted(
                                        &StyleSelector::from(pseudo.as_ref()).to_string(),
                                        b'&',
                                        "*",
                                    )
                                    .into_owned()
                                } else {
                                    name
                                },
                                inputs.file.to_string(),
                            );
                            // `None` when the enclosing at-rules can never match together.
                            let selector = at_rules.iter().try_fold(global, |selector, rule| {
                                StyleSelector::nest_at_rule(Some(&selector), rule.kind, &rule.query)
                            });
                            let extracted = selector.map_or_default(|selector| {
                                extract_style_from_expression(
                                    super::style_inputs::StyleInputs::with_values(
                                        ast_builder,
                                        inputs.values,
                                    ),
                                    None,
                                    &mut o.value,
                                    0,
                                    &Some(selector),
                                    LiteralHandling::ExpandResponsiveThemeToken,
                                )
                            });

                            // `@layer` names the layer of every other declaration,
                            // responsive and nested ones included
                            let mut extracted = extracted.styles;
                            extracted.retain(|style| {
                                !matches!(style, ExtractStyleProp::Static(ExtractStyleValue::Static(st)) if st.property() == "@layer")
                            });
                            if let Some(layer) = layer_name {
                                place_in_layer(&mut extracted, &layer);
                            }
                            yield_typography(&mut extracted);
                            styles.extend(extracted);
                        }
                    } else {
                        styles.push(unreadable_key(&o.key, false));
                    }
                }
                ObjectPropertyKind::SpreadProperty(o) => {
                    collect_global_styles(
                        ast_builder,
                        o.argument.get_inner_expression_mut(),
                        inputs,
                        at_rules,
                        styles,
                    );
                }
            }
        }
    } else {
        styles.extend(unreadable(expression).styles);
    }
}
