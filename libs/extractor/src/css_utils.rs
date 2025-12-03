use std::collections::BTreeMap;

use crate::utils::get_string_by_literal_expression;
use css::{
    optimize_multi_css_value::{check_multi_css_optimize, optimize_mutli_css_value},
    rm_css_comment::rm_css_comment,
    style_selector::StyleSelector,
};

use crate::utils::expression_to_code;
use oxc_ast::ast::TemplateLiteral;

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

/// Normalize whitespace while preserving string literals
fn normalize_whitespace_preserving_strings(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut string_placeholders = Vec::new();
    let mut placeholder_counter = 0;

    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' || ch == '`' {
            // Found start of string literal
            let quote = ch;
            let mut string_content = String::from(quote);
            let mut escaped = false;

            while let Some(&next_ch) = chars.peek() {
                chars.next();
                string_content.push(next_ch);

                if escaped {
                    escaped = false;
                } else if next_ch == '\\' {
                    escaped = true;
                } else if next_ch == quote {
                    // End of string literal
                    break;
                }
            }

            // Replace string literal with placeholder
            let placeholder = format!("__STRING_{}__", placeholder_counter);
            placeholder_counter += 1;
            string_placeholders.push((placeholder.clone(), string_content));
            result.push_str(&placeholder);
        } else {
            result.push(ch);
        }
    }

    // Normalize whitespace (newlines, tabs, multiple spaces)
    result = result.replace(['\n', '\t'], " ");
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }
    result = result.trim().to_string();

    // Restore string literals
    for (placeholder, original_string) in string_placeholders.iter().rev() {
        result = result.replace(placeholder, original_string);
    }

    result
}
pub fn css_to_style_literal<'a>(
    css: &TemplateLiteral<'a>,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<CssToStyleResult> {
    let mut styles = vec![];

    // If there are no expressions, just process quasis as static CSS
    if css.expressions.is_empty() {
        for quasi in css.quasis.iter() {
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
    // Use a format that won't conflict with actual CSS values
    let mut css_parts = Vec::new();
    let mut expression_map = std::collections::HashMap::new();

    for (i, quasi) in css.quasis.iter().enumerate() {
        css_parts.push(quasi.value.raw.to_string());

        // Add expression placeholder if not the last quasi
        if i < css.expressions.len() {
            // Use a unique placeholder format that CSS parser won't modify
            let placeholder = format!("__EXPR_{}__", i);
            expression_map.insert(placeholder.clone(), i);
            css_parts.push(placeholder);
        }
    }

    let combined_css = css_parts.join("");

    // Parse CSS to extract static styles
    let static_styles = css_to_style(&combined_css, level, selector);

    // Process each static style and check if it contains expression placeholders
    for style in static_styles {
        let value = style.value();

        // Find all placeholders in this value
        let mut found_placeholders = Vec::new();
        for (placeholder, &idx) in expression_map.iter() {
            if value.contains(placeholder) {
                found_placeholders.push((placeholder.clone(), idx));
            }
        }

        if !found_placeholders.is_empty() {
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
                        static_value =
                            static_value.replace(placeholder.as_str(), literal_value.as_str());
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
                    && value.trim() == placeholder.as_str()
                    && *idx < css.expressions.len()
                {
                    // Value is just the expression - use expression code directly
                    let expr = &css.expressions[*idx];

                    // Check if expression is a function (arrow function or function expression)
                    let is_function = matches!(
                        expr,
                        oxc_ast::ast::Expression::ArrowFunctionExpression(_)
                            | oxc_ast::ast::Expression::FunctionExpression(_)
                    );

                    let mut identifier = expression_to_code(expr);

                    // Normalize the code string
                    // 1. Normalize whitespace while preserving string literals
                    identifier = normalize_whitespace_preserving_strings(&identifier);
                    // 2. Normalize arrow function whitespace
                    identifier = identifier
                        .replace(" => ", "=>")
                        .replace(" =>", "=>")
                        .replace("=> ", "=>");
                    // 4. Normalize function expression formatting
                    // 5. Normalize quotes
                    if !is_function {
                        // For non-function expressions, convert property access quotes
                        // object["color"] -> object['color']
                        identifier = identifier.replace("[\"", "['").replace("\"]", "']");
                    } else {
                        // Normalize function() { } to function(){ }
                        identifier = identifier.replace("function() {", "function(){");
                        identifier = identifier.replace("function (", "function(");
                        // Remove trailing semicolon and spaces before closing brace
                        identifier = identifier.replace("; }", "}");
                        identifier = identifier.replace(" }", "}");

                        // Wrap function in parentheses if not already wrapped
                        // and add (rest) call
                        let trimmed = identifier.trim();
                        // Check if already wrapped in parentheses
                        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
                            identifier = format!("({})", trimmed);
                        }
                        // Add (rest) call
                        identifier = format!("{}(rest)", identifier);
                        // For function expressions, convert string literals in ternary operators
                        // This handles cases like: (props)=>props.b ? "a" : "b" -> (props)=>props.b ? 'a' : 'b'
                        // Use simple pattern matching for ternary operator string literals
                        // Pattern: ? "text" : "text" -> ? 'text' : 'text'
                        // We'll replace " with ' but only in the context of ternary operators
                        let mut result = String::new();
                        let mut chars = identifier.chars().peekable();
                        let mut in_ternary_string = false;

                        while let Some(ch) = chars.next() {
                            if ch == '?' || ch == ':' {
                                result.push(ch);
                                // Skip whitespace
                                while let Some(&' ') = chars.peek() {
                                    result.push(chars.next().unwrap());
                                }
                                // Check if next is a string literal
                                if let Some(&'"') = chars.peek() {
                                    in_ternary_string = true;
                                    result.push('\'');
                                    chars.next(); // consume the "
                                }
                            } else if in_ternary_string && ch == '"' {
                                // Check if this is a closing quote by looking ahead
                                let mut peeked = chars.clone();
                                // Skip whitespace
                                while let Some(&' ') = peeked.peek() {
                                    peeked.next();
                                }
                                // If next is : or ? or ) or } or end, it's a closing quote
                                if peeked.peek().is_none()
                                    || matches!(
                                        peeked.peek(),
                                        Some(&':') | Some(&'?') | Some(&')') | Some(&'}')
                                    )
                                {
                                    result.push('\'');
                                    in_ternary_string = false;
                                } else {
                                    // Not a closing quote, keep as is
                                    result.push(ch);
                                }
                            } else {
                                result.push(ch);
                            }
                        }
                        identifier = result;
                    }
                    identifier = identifier.trim().to_string();

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

                    // Sort placeholders by their position in reverse order to avoid index shifting
                    found_placeholders.sort_by(|(a_placeholder, _), (b_placeholder, _)| {
                        template_literal
                            .rfind(a_placeholder)
                            .cmp(&template_literal.rfind(b_placeholder))
                    });

                    // Replace each placeholder with the actual expression in template literal format
                    for (placeholder, idx) in &found_placeholders {
                        if *idx < css.expressions.len() {
                            let expr = &css.expressions[*idx];

                            // Check if expression is a function (arrow function or function expression)
                            let is_function = matches!(
                                expr,
                                oxc_ast::ast::Expression::ArrowFunctionExpression(_)
                                    | oxc_ast::ast::Expression::FunctionExpression(_)
                            );

                            let expr_code = expression_to_code(expr);
                            // Normalize the expression code while preserving string literals
                            let mut normalized_code =
                                normalize_whitespace_preserving_strings(&expr_code);

                            if is_function {
                                // Normalize arrow function whitespace
                                normalized_code = normalized_code
                                    .replace(" => ", "=>")
                                    .replace(" =>", "=>")
                                    .replace("=> ", "=>");
                                // Normalize function() { } to function(){ }
                                normalized_code =
                                    normalized_code.replace("function() {", "function(){");
                                normalized_code =
                                    normalized_code.replace("function (", "function(");
                                // Remove trailing semicolon and spaces before closing brace
                                normalized_code = normalized_code.replace("; }", "}");
                                normalized_code = normalized_code.replace(" }", "}");

                                // Wrap function in parentheses if not already wrapped
                                let trimmed = normalized_code.trim();
                                if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
                                    normalized_code = format!("({})", trimmed);
                                }
                                // Add (rest) call
                                normalized_code = format!("{}(rest)", normalized_code);

                                // For function expressions, convert string literals in ternary operators
                                let mut result = String::new();
                                let mut chars = normalized_code.chars().peekable();
                                let mut in_ternary_string = false;

                                while let Some(ch) = chars.next() {
                                    if ch == '?' || ch == ':' {
                                        result.push(ch);
                                        // Skip whitespace
                                        while let Some(&' ') = chars.peek() {
                                            result.push(chars.next().unwrap());
                                        }
                                        // Check if next is a string literal
                                        if let Some(&'"') = chars.peek() {
                                            in_ternary_string = true;
                                            result.push('\'');
                                            chars.next(); // consume the "
                                        }
                                    } else if in_ternary_string && ch == '"' {
                                        // Check if this is a closing quote by looking ahead
                                        let mut peeked = chars.clone();
                                        // Skip whitespace
                                        while let Some(&' ') = peeked.peek() {
                                            peeked.next();
                                        }
                                        // If next is : or ? or ) or } or end, it's a closing quote
                                        if peeked.peek().is_none()
                                            || matches!(
                                                peeked.peek(),
                                                Some(&':') | Some(&'?') | Some(&')') | Some(&'}')
                                            )
                                        {
                                            result.push('\'');
                                            in_ternary_string = false;
                                        } else {
                                            // Not a closing quote, keep as is
                                            result.push(ch);
                                        }
                                    } else {
                                        result.push(ch);
                                    }
                                }
                                normalized_code = result;
                            }

                            // Replace placeholder with ${expr} syntax
                            let expr_template = format!("${{{}}}", normalized_code);
                            template_literal =
                                template_literal.replace(placeholder.as_str(), &expr_template);
                        }
                    }

                    // Wrap in template literal backticks
                    let final_identifier = format!("`{}`", template_literal);

                    styles.push(CssToStyleResult::Dynamic(ExtractDynamicStyle::new(
                        style.property(),
                        style.level(),
                        &final_identifier,
                        style.selector().cloned(),
                    )));
                }
            }
        } else {
            // Check if property name contains a dynamic expression placeholder
            let property = style.property();

            if !expression_map.keys().any(|p| property.contains(p)) {
                // Static style
                styles.push(CssToStyleResult::Static(style));
            }

            // Property name is dynamic - skip for now as it's more complex
        }
    }

    styles
}

pub fn css_to_style(
    css: &str,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<ExtractStaticStyle> {
    let mut styles = vec![];
    let mut input = css;

    if input.contains("@media") {
        let media_inputs = input
            .split("@media")
            .flat_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(format!("@media{s}"))
                }
            })
            .collect::<Vec<_>>();
        if media_inputs.len() > 1 {
            for media_input in media_inputs {
                styles.extend(css_to_style(&media_input, level, selector));
            }
            return styles;
        }
    }

    if input.contains('{') {
        while let Some(start) = input.find('{') {
            // Check if there are properties before the selector
            let before_brace = &input[..start].trim();

            // Split by semicolon to find the last part which should be the selector
            let parts: Vec<&str> = before_brace.split(';').map(|s| s.trim()).collect();

            // Find the selector part (the last part that doesn't contain ':')
            // or if all parts contain ':', then the last part is the selector
            let (plain_props, selector_part) = if parts.len() > 1 {
                // Check if any part doesn't contain ':' (which would be a selector)
                let mut selector_idx = parts.len();
                for (i, part) in parts.iter().enumerate().rev() {
                    if !part.contains(':') || part.starts_with('&') || part.starts_with('@') {
                        selector_idx = i;
                        break;
                    }
                }

                // Math.min
                let (props, sel) = parts.split_at(parts.len().min(selector_idx));
                (props.join(";"), sel.join(";"))
            } else {
                ("".to_string(), before_brace.to_string())
            };

            // Process plain properties if any
            if !plain_props.is_empty() {
                styles.extend(css_to_style_block(&plain_props, level, selector));
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
            let sel = &if let Some(StyleSelector::Media { query, .. }) = selector {
                let local_sel = selector_part.trim().to_string();
                Some(StyleSelector::Media {
                    query: query.clone(),
                    selector: if local_sel == "&" {
                        None
                    } else {
                        Some(local_sel)
                    },
                })
            } else {
                let sel = selector_part.trim().to_string();
                if sel.starts_with("@media") {
                    Some(StyleSelector::Media {
                        query: sel.replace(" ", "").replace("and(", "and (")["@media".len()..]
                            .to_string(),
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
                if !remaining.is_empty() {
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
            } else {
                break;
            }
        }
    } else {
        styles.extend(css_to_style_block(input, level, selector));
    }

    styles.sort_by_key(|a| a.property().to_string());
    styles
}

fn css_to_style_block(
    css: &str,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<ExtractStaticStyle> {
    rm_css_comment(css)
        .split(";")
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                let mut iter = s.split(":").map(|s| s.trim());
                let property = iter.next().unwrap();
                let value = iter.next().unwrap();
                let value = if check_multi_css_optimize(property) {
                    optimize_mutli_css_value(value)
                } else {
                    value.to_string()
                };
                Some(ExtractStaticStyle::new(
                    property,
                    &value,
                    level,
                    selector.clone(),
                ))
            }
        })
        .collect()
}

pub fn keyframes_to_keyframes_style(keyframes: &str) -> BTreeMap<String, Vec<ExtractStaticStyle>> {
    let mut map = BTreeMap::new();
    let mut input = keyframes;

    while let Some(start) = input.find('{') {
        let key = input[..start].trim().to_string();
        let rest = &input[start + 1..];
        if let Some(end) = rest.find('}') {
            let block = &rest[..end];
            let mut styles = css_to_style(block, 0, &None);

            styles.sort_by_key(|a| a.property().to_string());
            map.insert(key, styles);
            input = &rest[end + 1..];
        } else {
            break;
        }
    }
    map
}

pub fn optimize_css_block(css: &str) -> String {
    rm_css_comment(css)
        .split("{")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>()
        .join("{")
        .split("}")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>()
        .join("}")
        .split(";")
        .map(|s| {
            let parts = s.split("{").collect::<Vec<&str>>();
            let first_part = if parts.len() == 1 {
                "".to_string()
            } else {
                format!("{}{{", parts.first().unwrap().trim())
            };
            let last_part = parts.last().unwrap().trim();
            if !last_part.contains(":") {
                format!("{first_part}{last_part}")
            } else {
                let mut iter = last_part.split(":");
                let property = iter.next().unwrap().trim();
                let value = iter.next().unwrap().trim();

                let value = if check_multi_css_optimize(property.split("{").last().unwrap()) {
                    optimize_mutli_css_value(value)
                } else {
                    value.to_string()
                };
                format!("{first_part}{property}:{value}")
            }
        })
        .collect::<Vec<String>>()
        .join(";")
        .trim()
        .replace(";}", "}")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use oxc_allocator::Allocator;
    use oxc_ast::ast::{Expression, Statement};
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    #[rstest]
    #[case("`background-color: red;`", vec![("background-color", "red", None)])]
    #[case("`background-color: ${color};`", vec![("background-color", "color", None)])]
    #[case("`background-color: ${color}`", vec![("background-color", "color", None)])]
    #[case("`background-color: ${color};color: blue;`", vec![("background-color", "color", None), ("color", "blue", None)])]
    #[case("`background-color: ${()=>\"arrow dynamic\"}`", vec![("background-color", "(()=>\"arrow dynamic\")(rest)", None)])]
    #[case("`background-color: ${()=>\"arrow dynamic\"};color: blue;`", vec![("background-color", "(()=>\"arrow dynamic\")(rest)", None), ("color", "blue", None)])]
    #[case("`color: blue;background-color: ${()=>\"arrow dynamic\"};`", vec![("color", "blue", None),("background-color", "(()=>\"arrow dynamic\")(rest)", None)])]
    #[case("`background-color: ${function(){ return \"arrow dynamic\"}}`", vec![("background-color", "(function(){ return \"arrow dynamic\"})(rest)", None)])]
    #[case("`background-color: ${function     ()      {          return \"arrow dynamic\"}              }`", vec![("background-color", "(function(){ return \"arrow dynamic\"})(rest)", None)])]
    #[case("`background-color: ${object.color}`", vec![("background-color", "object.color", None)])]
    #[case("`background-color: ${object['color']}`", vec![("background-color", "object['color']", None)])]
    #[case("`background-color: ${func()}`", vec![("background-color", "func()", None)])]
    #[case("`background-color: ${(props)=>props.b ? 'a' : 'b'}`", vec![("background-color", "((props)=>props.b ? 'a' : 'b')(rest)", None)])]
    #[case("`background-color: ${(props)=>props.b ? null : undefined}`", vec![("background-color", "((props)=>props.b ? null : undefined)(rest)", None)])]
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
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
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
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
    #[case("`width: ${1 + 1}px;`", vec![("width", "`${1 + 1}px`", None)])]
    #[case("`width: ${func(1)}px;`", vec![("width", "`${func(1)}px`", None)])]
    #[case("`width: ${func(1)}${2}px;`", vec![("width", "`${func(1)}${2}px`", None)])]
    #[case("`width: ${1}${2}px;`", vec![("width", "12px", None)])]
    #[case("`width: ${func(\n\t  1  ,   \n\t2\n)}px;`", vec![("width", "`${func(1,2)}px`", None)])]
    #[case("`width: ${func(\"  wow  \")}px;`", vec![("width", "`${func(\"  wow  \")}px`", None)])]
    #[case("`width: ${func(\"hello\\nworld\")}px;`", vec![("width", "`${func(\"hello\\nworld\")}px`", None)])]
    #[case("`width: ${func('test\\'quote')}px;`", vec![("width", "`${func(\"test'quote\")}px`", None)])]
    #[case("`width: ${(props)=>props.b ? \"hello\\\"world\" : \"test\"}px;`", vec![("width", "`${((props)=>props.b ? 'hello\\\"world' : 'test')(rest)}px`", None)])]
    #[case("`width: ${(props)=>props.b ? \"hello\\\"world\\\"more\" : \"test\"}px;`", vec![("width", "`${((props)=>props.b ? 'hello\\\"world\\\"more' : 'test')(rest)}px`", None)])]
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
        ":root,:section{background:red}"
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
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
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
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#000", Some(StyleSelector::Media {
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
        if styles.len() != expected.len() {
            panic!("styles.len() != expected.len()");
        }
        for (expected_key, expected_styles) in styles.iter() {
            let styles = expected_styles;
            let mut result: Vec<(&str, &str)> = styles
                .iter()
                .map(|prop| (prop.property(), prop.value()))
                .collect();
            result.sort();
            let mut expected_sorted = expected
                .iter()
                .find(|(k, _)| k == expected_key)
                .map(|(_, v)| v.clone())
                .unwrap();
            expected_sorted.sort();
            assert_eq!(result, expected_sorted);
        }
    }
}
