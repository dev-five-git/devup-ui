use std::error::Error;

use crate::parsing::goal_policy;
use oxc_allocator::Allocator;
use oxc_ast::ast::JSXExpressionContainer;
use oxc_codegen::CodegenReturn;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

/// JSX extraction outcomes used for final admission and sequence emission.
#[derive(Default)]
pub(crate) struct Outcome {
    pub(crate) sequences: bool,
    /// Whether the visit compiled the `css` prop of an element.
    pub(crate) compiled_css_prop: bool,
}

pub(crate) fn is_sequence(container: &JSXExpressionContainer<'_>) -> bool {
    container.expression.as_expression().is_some_and(|value| {
        matches!(
            crate::utils::unwrap_syntax_only(value),
            oxc_ast::ast::Expression::SequenceExpression(_)
        )
    })
}

/// Oxc prints JSX sequence expressions at its lowest precedence. Restore only
/// their missing parentheses, without adding evaluation or changing scope.
pub(crate) fn parenthesize(
    result: &mut CodegenReturn<'_>,
    source_type: SourceType,
) -> Result<(), Box<dyn Error>> {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, &result.code, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();
    if !goal_policy::has_diagnostics(&parsed) {
        return Ok(());
    }
    if parsed.fatal_error {
        return Err(format!("Cannot emit JSX: {:?}", parsed.diagnostics).into());
    }
    let mut insertions = Vec::new();
    for diagnostic in &parsed.diagnostics {
        if goal_policy::accepts_declaration_goal(
            &parsed,
            &diagnostic.to_string(),
            &diagnostic.labels,
        ) {
            continue;
        }
        if diagnostic.code.scope.as_deref() != Some("TS")
            || diagnostic.code.number.as_deref() != Some("18007")
            || diagnostic.labels.len() != 1
        {
            return Err(format!("Cannot emit JSX: {diagnostic}").into());
        }
        let label = &diagnostic.labels[0];
        let start = usize::try_from(label.offset())?;
        let end = start
            .checked_add(usize::try_from(label.len())?)
            .ok_or("JSX sequence span overflow")?;
        insertions.push((start, '('));
        insertions.push((end, ')'));
    }
    insertions.sort_unstable_by_key(|(offset, _)| *offset);
    let mut code = String::with_capacity(result.code.len() + insertions.len());
    let mut previous = 0;
    for &(offset, character) in &insertions {
        code.push_str(&result.code[previous..offset]);
        code.push(character);
        previous = offset;
    }
    code.push_str(&result.code[previous..]);
    validate(&code, source_type)?;
    if let Some(map) = result.map.take() {
        let offsets: Vec<_> = insertions.iter().map(|(offset, _)| *offset).collect();
        result.map = Some(crate::source_map::shift_generated(
            map,
            &result.code,
            &offsets,
        )?);
    }
    result.code = code;
    Ok(())
}

fn validate(code: &str, source_type: SourceType) -> Result<(), Box<dyn Error>> {
    let arena = Allocator::default();
    let checked = Parser::new(&arena, code, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();
    if goal_policy::has_diagnostics(&checked) {
        Err(format!(
            "Cannot emit JSX after grouping sequences: {:?}",
            checked.diagnostics
        )
        .into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "jsx_emission_tests.rs"]
mod tests;
