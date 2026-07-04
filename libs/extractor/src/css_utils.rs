use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::utils::{get_string_by_literal_expression, wrap_direct_call};
use css::{
    optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value},
    rm_css_comment::rm_css_comment,
    style_selector::{AtRuleKind, StyleSelector},
};
use oxc_allocator::Allocator;
use oxc_span::SPAN;

use crate::utils::expression_to_code;
use oxc_ast::ast::Expression;
use oxc_ast::ast::TemplateLiteral;
use oxc_ast::builder::AstBuilder;

use crate::extract_style::{
    extract_dynamic_style::ExtractDynamicStyle, extract_static_style::ExtractStaticStyle,
    extract_style_value::ExtractStyleValue,
};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub enum CssToStyleResult {
    Static(ExtractStaticStyle),
    Dynamic(ExtractDynamicStyle),
}

impl From<CssToStyleResult> for ExtractStyleValue {
    fn from(value: CssToStyleResult) -> Self {
        match value {
            CssToStyleResult::Static(style) => ExtractStyleValue::Static(style),
            CssToStyleResult::Dynamic(style) => ExtractStyleValue::Dynamic(style),
        }
    }
}

pub fn rm_last_semi_colon(code: &str) -> &str {
    code.trim_end_matches(';')
}

/// Convert a dynamic template-literal expression into identifier code,
/// wrapping arrow/function expressions in a direct call with `rest` and
/// trimming the trailing semicolon.
fn dynamic_expr_code<'a>(expr: &Expression<'a>, allocator: &'a Allocator) -> String {
    let is_function = matches!(
        expr,
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    );
    let code = if is_function {
        let ast_builder = AstBuilder::new(allocator);
        expression_to_code(&wrap_direct_call(
            &ast_builder,
            expr,
            &[Expression::new_identifier(SPAN, "rest", &ast_builder)],
        ))
    } else {
        expression_to_code(expr)
    };
    rm_last_semi_colon(&code).to_string()
}

pub fn css_to_style_literal(
    css: &TemplateLiteral<'_>,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<CssToStyleResult> {
    let mut styles = vec![];

    // If there are no expressions, just process quasis as static CSS
    if css.expressions.is_empty() {
        for quasi in &css.quasis {
            styles.extend(
                css_to_style(&quasi.value.raw, level, selector)
                    .into_iter()
                    .map(CssToStyleResult::Static),
            );
        }
        return styles;
    }

    // Process template literal with expressions
    // Template literal format: `text ${expr1} text ${expr2} text`
    // We need to parse CSS and identify where expressions are used

    // Build a combined CSS string with unique placeholders for expressions
    // Use a format that won't conflict with actual CSS values.
    // Build directly into one pre-sized buffer instead of collecting owned
    // `String`s into a `Vec` and `join`ing them: that avoids one heap
    // allocation per quasi, the `Vec` itself, and the `join` output copy.
    let quasi_len: usize = css.quasis.iter().map(|q| q.value.raw.len()).sum();
    // Each placeholder is `__EXPR_{i}__`; budget a small fixed width per
    // expression so the buffer rarely needs to grow.
    let mut combined_css = String::with_capacity(quasi_len + css.expressions.len() * 12);
    let mut expression_map =
        rustc_hash::FxHashMap::with_capacity_and_hasher(css.expressions.len(), Default::default());

    for (i, quasi) in css.quasis.iter().enumerate() {
        combined_css.push_str(&quasi.value.raw);

        // Add expression placeholder if not the last quasi
        if i < css.expressions.len() {
            // Use a unique placeholder format that CSS parser won't modify.
            // Write the placeholder straight into `combined_css` (no throwaway
            // `String`); only the owned map key needs a heap allocation, built once.
            let _ = write!(&mut combined_css, "__EXPR_{i}__");
            expression_map.insert(format!("__EXPR_{i}__"), i);
        }
    }

    // Parse CSS to extract static styles
    let static_styles = css_to_style(&combined_css, level, selector);

    // Shared allocator for AST builder used in dynamic expression processing
    let shared_allocator = Allocator::default();

    // Process each static style and check if it contains expression placeholders
    for style in static_styles {
        let value = style.value();

        // Fast path: every placeholder is `__EXPR_{i}__`, so a single shared-prefix probe
        // proves this declaration is purely static. Skip the per-style Vec allocation and the
        // full `expression_map` scan for the common no-placeholder case. This reproduces the
        // existing "found_placeholders empty && property has no placeholder" branch below.
        if !value.contains("__EXPR_") && !style.property().contains("__EXPR_") {
            styles.push(CssToStyleResult::Static(style));
            continue;
        }

        // Find all placeholders in this value. Borrow the placeholder key from
        // `expression_map` (owned + alive for this iteration) instead of cloning it;
        // it is only read back as `&str` in the literal-substitution loop below.
        let mut found_placeholders: Vec<(&str, usize)> = Vec::new();
        for (placeholder, &idx) in &expression_map {
            if value.contains(placeholder) {
                found_placeholders.push((placeholder.as_str(), idx));
            }
        }

        if found_placeholders.is_empty() {
            // Check if property name contains a dynamic expression placeholder
            let property = style.property();

            if !expression_map.keys().any(|p| property.contains(p)) {
                // Static style
                styles.push(CssToStyleResult::Static(style));
            }

            // Property name is dynamic - skip for now as it's more complex
        } else {
            // Check if all expressions are literals that can be statically evaluated

            let mut all_literals = true;
            let mut literal_values = Vec::new();

            let mut iter = found_placeholders.iter();
            while all_literals && let Some((_, idx)) = iter.next() {
                if *idx < css.expressions.len()
                    && let Some(literal_value) =
                        get_string_by_literal_expression(&css.expressions[*idx])
                {
                    literal_values.push((*idx, literal_value));
                } else {
                    all_literals = false;
                }
            }

            if all_literals {
                // All expressions are literals - replace placeholders with literal values to create static style
                let mut static_value = value.to_string();
                for (placeholder, idx) in &found_placeholders {
                    if let Some((_, literal_value)) = literal_values.iter().find(|(i, _)| i == idx)
                    {
                        static_value = static_value.replace(*placeholder, literal_value.as_str());
                    }
                }
                // Create a new static style with the evaluated value
                styles.push(CssToStyleResult::Static(ExtractStaticStyle::new(
                    style.property(),
                    &static_value,
                    style.level(),
                    style.selector().cloned(),
                )));
            } else {
                // Not all expressions are literals - need to create dynamic style
                // Check if value is just a placeholder (no surrounding text)
                if found_placeholders.len() == 1
                    && let (placeholder, idx) = &found_placeholders[0]
                    && value.trim() == *placeholder
                    && *idx < css.expressions.len()
                {
                    // Value is just the expression - use expression code directly
                    let identifier = dynamic_expr_code(&css.expressions[*idx], &shared_allocator);

                    styles.push(CssToStyleResult::Dynamic(ExtractDynamicStyle::new(
                        style.property(),
                        style.level(),
                        &identifier,
                        style.selector().cloned(),
                    )));
                } else {
                    // Value has surrounding text - need to create template literal
                    // Reconstruct the template literal by replacing placeholders with ${expr} syntax
                    // The value contains placeholders like "__EXPR_0__px", we need to convert to `${expr}px`

                    let mut template_literal = value.to_string();

                    // Sort placeholders by their position in reverse order to avoid index shifting.
                    // Decorate each placeholder with its `rfind` position ONCE (P scans total),
                    // sort by that precomputed key, then undecorate — instead of recomputing two
                    // full `rfind` scans inside every O(P log P) comparison.
                    let mut decorated: Vec<(Option<usize>, (&str, usize))> = found_placeholders
                        .iter()
                        .map(|entry| (template_literal.rfind(entry.0), *entry))
                        .collect();
                    decorated.sort_by_key(|(pos, _)| *pos);
                    for (dst, (_, entry)) in found_placeholders.iter_mut().zip(decorated) {
                        *dst = entry;
                    }

                    // Replace each placeholder with the actual expression in template literal format
                    for (placeholder, idx) in &found_placeholders {
                        if *idx < css.expressions.len() {
                            let expr_code =
                                dynamic_expr_code(&css.expressions[*idx], &shared_allocator);
                            // Replace placeholder with ${expr} syntax
                            template_literal = template_literal
                                .replace(*placeholder, &format!("${{{expr_code}}}"));
                        }
                    }

                    // Wrap in template literal backticks
                    let final_identifier = format!("`{template_literal}`");

                    styles.push(CssToStyleResult::Dynamic(ExtractDynamicStyle::new(
                        style.property(),
                        style.level(),
                        &final_identifier,
                        style.selector().cloned(),
                    )));
                }
            }
        }
    }

    styles
}

const AT_RULES: [(&str, AtRuleKind); 3] = [
    ("@media", AtRuleKind::Media),
    ("@supports", AtRuleKind::Supports),
    ("@container", AtRuleKind::Container),
];

pub fn css_to_style(
    css: &str,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<ExtractStaticStyle> {
    let mut styles = vec![];
    let mut input = css;

    // Split by at-rules (@media, @supports, @container) to handle multiple at-rules in a single input.
    // Every at-rule prefix begins with `@`, so a single `@` byte scan is a sound necessary-condition
    // guard: if the input has no `@` at all (the overwhelmingly common declaration block), none of the
    // three prefixes can match, so skip all three `input.contains(at_rule)` substring scans entirely.
    if input.contains('@') {
        for (at_rule, _) in AT_RULES {
            // Only the multi-segment case recurses. Walk the non-empty trimmed `@rule` segments with
            // a single `split` pass (dropping the earlier separate `count` scan + identical `collect`
            // scan) via a peekable iterator: confirm a *second* non-empty segment before allocating,
            // so the common single-`@media`/`@supports`/`@container` block (already dispatched by an
            // outer recursion level) still skips materializing a throwaway `Vec<String>`.
            // `split(at_rule)` already performs the substring scan that a separate
            // `input.contains(at_rule)` guard would do, and the peekable `first`/`second`
            // check below already no-ops when the prefix is absent (a single segment yields
            // `second == None`). So run `split` directly and drop the redundant O(n)
            // `contains` scan — byte-identical behavior, one fewer full-string scan per
            // at-rule kind on every `@`-bearing input.
            let mut segments = input.split(at_rule).filter_map(|s| {
                let s = s.trim();
                (!s.is_empty()).then_some(s)
            });
            if let Some(first) = segments.next()
                && let Some(second) = segments.next()
            {
                // Re-attach the known `at_rule` prefix to each segment with a presized
                // `String` + two `push_str` instead of `format!`, which pulls in the
                // `Arguments` formatting machinery and its grow path. Both lengths are
                // known up front, so a single exact allocation suffices. Byte-identical
                // to `format!("{at_rule}{seg}")`.
                let join_at = |seg: &str| {
                    let mut s = String::with_capacity(at_rule.len() + seg.len());
                    s.push_str(at_rule);
                    s.push_str(seg);
                    s
                };
                styles.extend(css_to_style(&join_at(first), level, selector));
                styles.extend(css_to_style(&join_at(second), level, selector));
                for rest in segments {
                    styles.extend(css_to_style(&join_at(rest), level, selector));
                }
                return styles;
            }
        }
    }

    if input.contains('{') {
        while let Some(start) = input.find('{') {
            // Check if there are properties before the selector
            let before_brace = &input[..start].trim();

            // The overwhelmingly common case has no `;`-separated plain props before the
            // selector (e.g. `&:hover { ... }`), which maps to the single-part `else`
            // branch below. Only build the `Vec<&str>` split when a `;` is actually present.
            let (plain_props, selector_part): (&str, &str) = if before_brace.contains(';') {
                // Split by semicolon to find the last part which should be the selector
                let parts: Vec<&str> = before_brace.split(';').map(str::trim).collect();

                // Find the selector part (the last part that doesn't contain ':')
                // or if all parts contain ':', then the last part is the selector
                if parts.len() > 1 {
                    // Check if any part doesn't contain ':' (which would be a selector)
                    let mut selector_idx = parts.len();
                    for (i, part) in parts.iter().enumerate().rev() {
                        if !part.contains(':') || part.starts_with('&') || part.starts_with('@') {
                            selector_idx = i;
                            break;
                        }
                    }

                    // Borrow the props/selector partition directly from `before_brace`
                    // instead of allocating two `join(";")` Strings. The split boundary is the
                    // byte offset of the `selector_idx`-th `;` (parts are `;`-separated), so
                    // `[..boundary]` is the props run and `[boundary + 1..]` the selector run.
                    // `css_to_style_block` re-splits/re-trims each `;` part on the props side and
                    // the selector side is `.trim()`ed downstream, so the parsed output stays
                    // byte-identical to the previous `join(";")` form.
                    let boundary = before_brace
                        .match_indices(';')
                        .nth(selector_idx - 1)
                        .map(|(idx, _)| idx);
                    match boundary {
                        Some(b) => (before_brace[..b].trim(), before_brace[b + 1..].trim()),
                        // `selector_idx == 0`: no props, whole `before_brace` is the selector.
                        None => ("", before_brace),
                    }
                } else {
                    ("", before_brace)
                }
            } else {
                ("", before_brace)
            };

            // Process plain properties if any
            if !plain_props.is_empty() {
                styles.extend(css_to_style_block(plain_props, level, selector));
            }

            let rest = &input[start + 1..];

            // Find the matching closing brace by counting braces
            let mut brace_count = 1;
            let mut end = 0;
            for (i, ch) in rest.char_indices() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            // If we didn't find a matching brace, use the first '}' as fallback
            if brace_count > 0 {
                end = rest.find('}').unwrap_or(rest.len());
            }
            let block = &rest[..end];
            let sel = &if let Some(StyleSelector::At { kind, query, .. }) = selector {
                let local_sel = selector_part.trim().to_string();
                Some(StyleSelector::At {
                    kind: *kind,
                    query: query.clone(),
                    selector: if local_sel == "&" {
                        None
                    } else {
                        Some(local_sel)
                    },
                })
            } else {
                let sel = selector_part.trim().to_string();
                if let Some((prefix, kind)) =
                    AT_RULES.iter().find(|(prefix, _)| sel.starts_with(prefix))
                {
                    // The prefix contains neither spaces nor "and(", so slicing it off
                    // first is equivalent to slicing after the replaces. Strip spaces into
                    // one pre-sized buffer, then a single "and(" -> "and (" normalization.
                    let rest = &sel[prefix.len()..];
                    let mut query = String::with_capacity(rest.len());
                    for ch in rest.chars() {
                        if ch != ' ' {
                            query.push(ch);
                        }
                    }
                    Some(StyleSelector::At {
                        kind: *kind,
                        query: query.replace("and(", "and ("),
                        selector: None,
                    })
                } else if sel.is_empty() {
                    selector.clone()
                } else {
                    Some(StyleSelector::Selector(sel))
                }
            };
            let block = if block.contains('{') {
                css_to_style(block, level, sel)
            } else {
                css_to_style_block(block, level, sel)
            };

            // Find the matching closing brace
            let closing_brace_pos = start + 1 + end;

            // Process the block
            styles.extend(block);

            // Update input to continue processing after the closing brace
            // Check if there's more content after the closing brace
            if closing_brace_pos + 1 < input.len() {
                let remaining = &input[closing_brace_pos + 1..].trim();
                if remaining.is_empty() {
                    break;
                }
                // If there's remaining text after the closing brace, process it
                // This handles cases like "} color: blue;"
                if remaining.contains('{') {
                    // If it contains '{', continue the loop
                    input = remaining;
                } else {
                    // If it doesn't contain '{', process it as a block and break
                    styles.extend(css_to_style_block(remaining, level, selector));
                    break;
                }
            } else {
                break;
            }
        }
    } else {
        styles.extend(css_to_style_block(input, level, selector));
    }

    // A single declaration (or none) is trivially ordered, so skip the comparison
    // sort's setup entirely for the very common single-property case. The multi-source
    // merge (multiple `@rule`/`{}` segments) and the multi-declaration block still need
    // the property sort to keep the emitted CSS order byte-identical.
    if styles.len() > 1 {
        styles.sort_unstable_by(|a, b| a.property().cmp(b.property()));
    }
    styles
}

fn css_to_style_block(
    css: &str,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<ExtractStaticStyle> {
    let cleaned = rm_css_comment(css);
    // Presize to an upper bound (`;`-count + 1 = max declarations). The dominant
    // single-declaration block (template/styled) contains no `;`, so gate the full
    // `matches(';').count()` byte scan behind a cheap `contains(';')` check: single
    // blocks presize to 1 without a second traversal, while multi-declaration blocks
    // still presize exactly to skip the intermediate grow-reallocs.
    let cap = if cleaned.contains(';') {
        cleaned.matches(';').count() + 1
    } else {
        1
    };
    let mut styles = Vec::with_capacity(cap);
    for s in cleaned.split(';') {
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        let Some((property, value)) = s.split_once(':') else {
            continue;
        };
        let property = property.trim();
        let value = value.trim();
        let value = if check_multi_css_optimize(property) {
            optimize_multi_css_value(value)
        } else {
            value.to_string()
        };
        styles.push(ExtractStaticStyle::new(
            property,
            &value,
            level,
            selector.clone(),
        ));
    }
    styles
}

pub fn keyframes_to_keyframes_style(keyframes: &str) -> BTreeMap<String, Vec<ExtractStaticStyle>> {
    let mut map = BTreeMap::new();
    let mut input = keyframes;

    while let Some(start) = input.find('{') {
        let key = input[..start].trim().to_string();
        let rest = &input[start + 1..];
        if let Some(end) = rest.find('}') {
            let block = &rest[..end];
            // css_to_style already returns styles sorted by property.
            map.insert(key, css_to_style(block, 0, &None));
            input = &rest[end + 1..];
        } else {
            break;
        }
    }
    map
}

pub fn optimize_css_block(css: &str) -> String {
    // First pass: remove comments and normalize whitespace around structural chars
    let cleaned = rm_css_comment(css);

    // Second pass: trim around {, }, ; and optimize declarations in one go
    let mut result = String::with_capacity(cleaned.len());
    // Trim whitespace around every `{` and `}` boundary in ONE pass, writing into a
    // single buffer. This is equivalent to the previous split('{')-then-split('}')
    // rebuild but avoids the intermediate whole-string allocation: each segment between
    // two structural chars is trimmed once and the consumed `{`/`}` is re-emitted.
    let trimmed = {
        let mut buf = String::with_capacity(cleaned.len());
        let mut segment_start = 0;
        let bytes = cleaned.as_bytes();
        for (idx, &b) in bytes.iter().enumerate() {
            if b == b'{' || b == b'}' {
                buf.push_str(cleaned[segment_start..idx].trim());
                buf.push(b as char);
                segment_start = idx + 1;
            }
        }
        buf.push_str(cleaned[segment_start..].trim());
        buf
    };

    let mut first_segment = true;
    for s in trimmed.split(';') {
        if !first_segment {
            result.push(';');
        }
        first_segment = false;

        let last_part = if let Some((prefix, last_part)) = s.rsplit_once('{') {
            append_brace_prefix(&mut result, prefix);
            last_part.trim()
        } else {
            s.trim()
        };

        if let Some((property, value)) = last_part.split_once(':') {
            let property = property.trim();
            let value = value.trim();

            let property_name = property
                .rsplit_once('{')
                .map_or(property, |(_, property_name)| property_name);
            let optimized_value = if check_multi_css_optimize(property_name) {
                optimize_multi_css_value(value)
            } else {
                value.to_string()
            };
            result.push_str(property);
            result.push(':');
            result.push_str(&optimized_value);
        } else {
            result.push_str(last_part);
        }
    }

    trim_string_in_place(&mut result);
    if result.is_empty() {
        return String::new();
    }
    remove_semicolon_before_closing_brace(&mut result);
    result
}

fn append_brace_prefix(result: &mut String, prefix: &str) {
    for (idx, part) in prefix.split('{').enumerate() {
        if idx > 0 {
            result.push('{');
        }
        result.push_str(part.trim());
    }
    result.push('{');
}

fn trim_string_in_place(value: &mut String) {
    let trimmed_start = value.len() - value.trim_start().len();
    if trimmed_start > 0 {
        value.drain(..trimmed_start);
    }

    let trimmed_len = value.trim_end().len();
    value.truncate(trimmed_len);
}

fn remove_semicolon_before_closing_brace(value: &mut String) {
    if !value.contains(";}") {
        return;
    }
    // Single forward pass: drop every run of ';' that sits directly before '}'.
    let mut out = String::with_capacity(value.len());
    let mut rest = value.as_str();
    while let Some(pos) = rest.find(';') {
        out.push_str(&rest[..pos]);
        let after_run = rest[pos..].trim_start_matches(';');
        if !after_run.starts_with('}') {
            out.push_str(&rest[pos..rest.len() - after_run.len()]);
        }
        rest = after_run;
    }
    out.push_str(rest);
    *value = out;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    use oxc_allocator::Allocator;
    use oxc_ast::ast::{Expression, Statement};
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    #[rstest]
    #[case("a{color:red;}", "a{color:red}")]
    #[case("a{color:red;;}", "a{color:red}")]
    #[case(";}", "}")]
    #[case(";;;a;}", ";;;a}")]
    #[case("a{color:red;top:0}", "a{color:red;top:0}")]
    #[case("", "")]
    fn test_remove_semicolon_before_closing_brace(#[case] input: &str, #[case] expected: &str) {
        let mut value = input.to_string();
        remove_semicolon_before_closing_brace(&mut value);
        assert_eq!(value, expected);
    }

    #[rstest]
    #[case("`background-color: red;`", vec![("background-color", "red", None)])]
    #[case("`background-color: ${color};`", vec![("background-color", "color", None)])]
    #[case("`background-color: ${color}`", vec![("background-color", "color", None)])]
    #[case("`background-color: ${color};color: blue;`", vec![("background-color", "color", None), ("color", "blue", None)])]
    #[case("`background-color: ${()=>\"arrow dynamic\"}`", vec![("background-color", "(()=>`arrow dynamic`)(rest)", None)])]
    #[case("`background-color: ${()=>\"arrow dynamic\"};color: blue;`", vec![("background-color", "(()=>`arrow dynamic`)(rest)", None), ("color", "blue", None)])]
    #[case("`color: blue;background-color: ${()=>\"arrow dynamic\"};`", vec![("color", "blue", None),("background-color", "(()=>`arrow dynamic`)(rest)", None)])]
    #[case("`background-color: ${function(){ return \"arrow dynamic\"}}`", vec![("background-color", "(function(){return`arrow dynamic`})(rest)", None)])]
    #[case("`background-color: ${function     ()      {          return \"arrow dynamic\"}              }`", vec![("background-color",  "(function(){return`arrow dynamic`})(rest)", None)])]
    #[case("`background-color: ${object.color}`", vec![("background-color", "object.color", None)])]
    #[case("`background-color: ${object['color']}`", vec![("background-color", "object[`color`]", None)])]
    #[case("`background-color: ${func()}`", vec![("background-color", "func()", None)])]
    #[case("`background-color: ${(props)=>props.b ? 'a' : 'b'}`", vec![("background-color", "(props=>props.b?`a`:`b`)(rest)", None)])]
    #[case("`background-color: ${(props)=>props.b ? null : undefined}`", vec![("background-color", "(props=>props.b?null:undefined)(rest)", None)])]
    #[case(
        "`color: red; background: blue;`",
        vec![
            ("color", "red", None),
            ("background", "blue", None),
        ]
    )]
    #[case(
        "`margin:0;padding:0;`",
        vec![
            ("margin", "0", None),
            ("padding", "0", None),
        ]
    )]
    #[case(
        "`font-size: 16px;`",
        vec![
            ("font-size", "16px", None),
        ]
    )]
    #[case(
        "`border: 1px solid #000; color: #fff;`",
        vec![
            ("border", "1px solid #000", None),
            ("color", "#FFF", None),
        ]
    )]
    #[case(
        "``",
        vec![]
    )]
    #[case(
        "`@media (min-width: 768px) {
            border: 1px solid #000;
            color: #fff;
        }`",
        vec![
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "`@media (min-width: 768px) and (max-width: 1024px) {
            border: 1px solid #000;
            color: #fff;
        }

        @media (min-width: 768px) {
            border: 1px solid #000;
            color: #fff;
        }`",
        vec![
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "`@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover,   &:active, &:nth-child(2) {
                border: 1px solid #000;
                color: #000;
            }
        }`",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
        ]
    )]
    #[case(
        "`@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }`",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
        ]
    )]
    #[case(
        "`@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }`",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
        ]
    )]
    #[case(
        "`@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
            border: 1px solid #000;
            color: #000;
        }`",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "`@media (min-width: 768px) {
            & {
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
        }`",
        vec![]
    )]
    // @supports test cases
    #[case(
        "`@supports (display: grid) {
            display: grid;
            grid-template-columns: 1fr 1fr;
        }`",
        vec![
            ("display", "grid", Some(StyleSelector::At {
                kind: AtRuleKind::Supports,
                query: "(display:grid)".to_string(),
                selector: None,
            })),
            ("grid-template-columns", "1fr 1fr", Some(StyleSelector::At {
                kind: AtRuleKind::Supports,
                query: "(display:grid)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "`@supports (display: flex) {
            &:hover {
                display: flex;
            }
        }`",
        vec![
            ("display", "flex", Some(StyleSelector::At {
                kind: AtRuleKind::Supports,
                query: "(display:flex)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
        ]
    )]
    #[case(
        "`@supports not (display: grid) {
            display: block;
        }`",
        vec![
            ("display", "block", Some(StyleSelector::At {
                kind: AtRuleKind::Supports,
                query: "not(display:grid)".to_string(),
                selector: None,
            })),
        ]
    )]
    // @container test cases
    #[case(
        "`@container (min-width: 768px) {
            padding: 10px;
        }`",
        vec![
            ("padding", "10px", Some(StyleSelector::At {
                kind: AtRuleKind::Container,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "`@container sidebar (min-width: 400px) {
            display: flex;
        }`",
        vec![
            ("display", "flex", Some(StyleSelector::At {
                kind: AtRuleKind::Container,
                query: "sidebar(min-width:400px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "`ul { font-family: 'Roboto Hello',       sans-serif; }`",
        vec![
            ("font-family", "\"Roboto Hello\",sans-serif", Some(StyleSelector::Selector("ul".to_string()))),
        ]
    )]
    #[case(
        "`&:hover { background-color: red; }`",
        vec![
            ("background-color", "red", Some(StyleSelector::Selector("&:hover".to_string()))),
        ]
    )]
    #[case(
        "`background-color: red; &:hover { background-color: red; }`",
        vec![
            ("background-color", "red", None),
            ("background-color", "red", Some(StyleSelector::Selector("&:hover".to_string()))),
        ]
    )]
    #[case(
        "`background-color: red; &:hover { background-color: red; } color: blue;`",
        vec![
            ("background-color", "red", None),
            ("background-color", "red", Some(StyleSelector::Selector("&:hover".to_string()))),
            ("color", "blue", None),
        ]
    )]
    #[case(
        "`background-color: red; &:hover { background-color: red; } color: blue; &:active { background-color: blue; }`",
        vec![
            ("background-color", "red", None),
            ("background-color", "red", Some(StyleSelector::Selector("&:hover".to_string()))),
            ("color", "blue", None),
            ("background-color", "blue", Some(StyleSelector::Selector("&:active".to_string()))),
        ]
    )]
    #[case(
        "`background-color: red; &:hover { background-color: red; } color: blue; &:active { background-color: blue; } transform: rotate(90deg);`",
        vec![
            ("background-color", "red", None),
            ("background-color", "red", Some(StyleSelector::Selector("&:hover".to_string()))),
            ("color", "blue", None),
            ("background-color", "blue", Some(StyleSelector::Selector("&:active".to_string()))),
            ("transform", "rotate(90deg)", None),
        ]
    )]
    #[case("`width: ${1}px;`", vec![("width", "1px", None)])]
    #[case("`width: ${\"1\"}px;`", vec![("width", "1px", None)])]
    #[case("`width: ${'1'}px;`", vec![("width", "1px", None)])]
    #[case("`width: ${`1`}px;`", vec![("width", "1px", None)])]
    #[case("`width: ${\"1px\"};`", vec![("width", "1px", None)])]
    #[case("`width: ${'1px'};`", vec![("width", "1px", None)])]
    #[case("`width: ${`1px`};`", vec![("width", "1px", None)])]
    #[case("`width: ${1 + 1}px;`", vec![("width", "`${1+1}px`", None)])]
    #[case("`width: ${func(1)}px;`", vec![("width", "`${func(1)}px`", None)])]
    #[case("`width: ${func(1)}${2}px;`", vec![("width", "`${func(1)}${2}px`", None)])]
    #[case("`width: ${1}${2}px;`", vec![("width", "12px", None)])]
    #[case("`width: ${func(\n\t  1  ,   \n\t2\n)}px;`", vec![("width", "`${func(1,2)}px`", None)])]
    #[case("`width: ${func(\"  wow  \")}px;`", vec![("width", "`${func(`  wow  `)}px`", None)])]
    #[case("`width: ${func(\"hello\\nworld\")}px;`", vec![("width", "`${func(`hello\nworld`)}px`", None)])]
    #[case("`width: ${func('test\\'quote')}px;`", vec![("width", "`${func(`test'quote`)}px`", None)])]
    #[case("`width: ${(props)=>props.b ? \"hello\\\"world\" : \"test\"}px;`", vec![("width", "`${(props=>props.b?`hello\"world`:`test`)(rest)}px`", None)])]
    #[case("`width: ${(props)=>props.b ? \"hello\\\"world\\\"more\" : \"test\"}px;`", vec![("width", "`${(props=>props.b?`hello\"world\"more`:`test`)(rest)}px`", None)])]
    #[case("`width: ${(props)=>props.b ? \"hello\" + \"world\" : \"test\"}px;`", vec![("width", "`${(props=>props.b?`hello`+`world`:`test`)(rest)}px`", None)])]
    #[case("`width: ${function(props){return props.b}}px;`", vec![("width", "`${(function(props){return props.b})(rest)}px`", None)])]
    // wrong cases
    #[case(
        "`@media (min-width: 768px) {
            & {
        `",
        vec![]
    )]
    fn test_css_to_style_literal(
        #[case] input: &str,
        #[case] expected: Vec<(&str, &str, Option<StyleSelector>)>,
    ) {
        // parse template literal code
        let allocator = Allocator::default();
        let css = Parser::new(&allocator, input, SourceType::ts()).parse();
        if let Statement::ExpressionStatement(expr) = &css.program.body[0]
            && let Expression::TemplateLiteral(tmp) = &expr.expression
        {
            let styles = css_to_style_literal(tmp, 0, &None);
            let mut result: Vec<(&str, &str, Option<StyleSelector>)> = styles
                .iter()
                .map(|prop| match prop {
                    CssToStyleResult::Static(style) => {
                        (style.property(), style.value(), style.selector().cloned())
                    }
                    CssToStyleResult::Dynamic(dynamic) => (
                        dynamic.property(),
                        dynamic.identifier(),
                        dynamic.selector().cloned(),
                    ),
                })
                .collect();
            result.sort();
            let mut expected_sorted = expected.clone();
            expected_sorted.sort();
            assert_eq!(result, expected_sorted);
        } else {
            panic!("not a template literal");
        }
    }

    #[rstest]
    #[case(
        "div{
        /* comment */
        background-color: red;
        /* color: blue; */
    }",
        "div{background-color:red}"
    )]
    #[case(
        "/*div{
        background-color: red;
    }*/",
        ""
    )]
    #[case(
        "       img      {       background-color    :       red;      }     ",
        "img{background-color:red}"
    )]
    #[case(
        "       img      {       background-color    :       red;          color     :          blue;      }     ",
        "img{background-color:red;color:blue}"
    )]
    #[case("div{margin : 0 ; padding : 0 ; }", "div{margin:0;padding:0}")]
    #[case(
        "a { text-decoration : none ; color : black ; }",
        "a{text-decoration:none;color:black}"
    )]
    #[case("body{background: #fff;}", "body{background:#fff}")]
    #[case(
        "h1{ font-size : 2rem ; font-weight : bold ; }",
        "h1{font-size:2rem;font-weight:bold}"
    )]
    #[case("span { }", "span{}")]
    #[case("p{color:blue;}", "p{color:blue}")]
    #[case(
        "ul { list-style : none ; margin : 0 ; padding : 0 ; }",
        "ul{list-style:none;margin:0;padding:0}"
    )]
    #[case(
        "ul { font-family: 'Roboto',       sans-serif; }",
        "ul{font-family:Roboto,sans-serif}"
    )]
    #[case(
        "ul { font-family: \"Roboto Hello\",       sans-serif; }",
        "ul{font-family:\"Roboto Hello\",sans-serif}"
    )]
    #[case("section{  }", "section{}")]
    #[case(":root{   }", ":root{}")]
    #[case(":root{ background: red; }", ":root{background:red}")]
    #[case(
        ":root, :section{ background: red; }",
        concat!(":root,:section", "{background:red}")
    )]
    #[case(
        "@supports (display: grid) { .grid { gap : 1rem ; } }",
        concat!("@supports (display: grid)", "{.grid", "{gap:1rem}}")
    )]
    #[case("*:hover{ background: red; }", "*:hover{background:red}")]
    #[case(":root {color-scheme: light dark }", ":root{color-scheme:light dark}")]
    fn test_optimize_css_block(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_css_block(input), expected);
    }

    #[rstest]
    #[case(
        "color: red; background: blue;",
        vec![
            ("color", "red", None),
            ("background", "blue", None),
        ]
    )]
    #[case(
        "margin:0;padding:0;",
        vec![
            ("margin", "0", None),
            ("padding", "0", None),
        ]
    )]
    #[case(
        "font-size: 16px;",
        vec![
            ("font-size", "16px", None),
        ]
    )]
    #[case(
        "border: 1px solid #000; color: #fff;",
        vec![
            ("border", "1px solid #000", None),
            ("color", "#FFF", None),
        ]
    )]
    #[case(
        "",
        vec![]
    )]
    #[case(
        "@media (min-width: 768px) {
            border: 1px solid #000;
            color: #fff;
        }",
        vec![
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) and (max-width: 1024px) {
            border: 1px solid #000;
            color: #fff;
        }
        
        @media (min-width: 768px) {
            border: 1px solid #000;
            color: #fff;
        }",
        vec![
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover,   &:active, &:nth-child(2) {
                border: 1px solid #000;
                color: #000;
            }
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
            border: 1px solid #000;
            color: #000;
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#000", Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
        }",
        vec![]
    )]
    #[case(
        "ul { font-family: 'Roboto Hello',       sans-serif; }",
        vec![
            ("font-family", "\"Roboto Hello\",sans-serif", Some(StyleSelector::Selector("ul".to_string()))),
        ]
    )]
    #[case(
        "div { color: red; ; { background: blue; } }",
        vec![
            ("color", "red", Some(StyleSelector::Selector("div".to_string()))),
            ("background", "blue", Some(StyleSelector::Selector("div".to_string()))),
        ]
    )]
    fn test_css_to_style(
        #[case] input: &str,
        #[case] expected: Vec<(&str, &str, Option<StyleSelector>)>,
    ) {
        let styles = css_to_style(input, 0, &None);
        let mut result: Vec<(&str, &str, Option<StyleSelector>)> = styles
            .iter()
            .map(|prop| (prop.property(), prop.value(), prop.selector().cloned()))
            .collect();
        result.sort();
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        assert_eq!(result, expected_sorted);
    }

    #[rstest]
    #[case(
        "to {\nbackground-color:red;\n}\nfrom {\nbackground-color:blue;\n}",
        vec![
            ("to", vec![("background-color", "red")]),
            ("from", vec![("background-color", "blue")]),
        ],
    )]
    #[case(
        "0% { opacity: 0; }\n100% { opacity: 1; }",
        vec![
            ("0%", vec![("opacity", "0")]),
            ("100%", vec![("opacity", "1")]),
        ],
    )]
    #[case(
        "from { left: 0; }\nto { left: 100px; }",
        vec![
            ("from", vec![("left", "0")]),
            ("to", vec![("left", "100px")]),
        ],
    )]
    #[case(
        "50% { color: red; background: blue; }",
        vec![
            ("50%", vec![("color", "red"), ("background", "blue")]),
        ],
    )]
    #[case(
        "",
        vec![],
    )]
    #[case(
        "50% { color: red        ; background: blue; }",
        vec![
            ("50%", vec![("color", "red"), ("background", "blue")]),
        ],
    )]
    // comment case
    #[case(
        "50% { color: red; /*background: blue;*/ }",
        vec![
            ("50%", vec![("color", "red")]),
        ],
    )]
    // error case
    #[case(
        "50% { color: red        ; background: blue ",
        vec![
        ],
    )]
    fn test_keyframes_to_keyframes_style(
        #[case] input: &str,
        #[case] expected: Vec<(&str, Vec<(&str, &str)>)>,
    ) {
        let styles = keyframes_to_keyframes_style(input);
        assert!(
            styles.len() == expected.len(),
            "styles.len() != expected.len()"
        );
        for (expected_key, expected_styles) in &styles {
            let styles = expected_styles;
            let mut result: Vec<(&str, &str)> = styles
                .iter()
                .map(|prop| (prop.property(), prop.value()))
                .collect();
            result.sort_unstable();
            let mut expected_sorted = expected
                .iter()
                .find(|(k, _)| k == expected_key)
                .map(|(_, v)| v.clone())
                .unwrap();
            expected_sorted.sort_unstable();
            assert_eq!(result, expected_sorted);
        }
    }

    #[rstest]
    #[case("  hello", "hello")]
    #[case("\t\nhello", "hello")]
    #[case("  hello  ", "hello")]
    #[case("hello  ", "hello")]
    #[case("hello", "hello")]
    #[case("", "")]
    #[case("   ", "")]
    fn test_trim_string_in_place(#[case] input: &str, #[case] expected: &str) {
        let mut value = input.to_string();
        trim_string_in_place(&mut value);
        assert_eq!(value, expected);
    }
}
