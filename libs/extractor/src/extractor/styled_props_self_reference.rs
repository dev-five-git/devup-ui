//! Resolve a named whole-rule callback's own binding before changing its return value.

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{
    ast::{Expression, ExpressionStatement, Program, Statement},
    builder::AstBuilder,
};
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SPAN, SourceType};

use crate::utils::{build_time_error, readable_code, unwrap_syntax_only};

pub(super) fn error(style: &Expression<'_>) -> Option<(u32, String)> {
    let Expression::FunctionExpression(function) = unwrap_syntax_only(style) else {
        return None;
    };
    let name = function.id.as_ref()?.name.as_str();
    // A fresh clone keeps semantic analysis from replacing the enclosing module's IDs.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let body = oxc_allocator::Vec::from_array_in(
        [Statement::ExpressionStatement(ExpressionStatement::boxed(
            SPAN,
            style.clone_in(&allocator),
            &builder,
        ))],
        &builder,
    );
    let program = Program::new(
        SPAN,
        SourceType::tsx(),
        "",
        oxc_allocator::Vec::new_in(&builder),
        None,
        oxc_allocator::Vec::new_in(&builder),
        body,
        &builder,
    );
    let semantic = SemanticBuilder::new().build(&program).semantic;
    let Statement::ExpressionStatement(statement) = program.body.first()? else {
        return None;
    };
    let Expression::FunctionExpression(function) = unwrap_syntax_only(&statement.expression) else {
        return None;
    };
    let symbol = function.id.as_ref()?.symbol_id.get()?;
    let scoping = semantic.scoping();
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .any(|reference| scoping.get_reference(*reference).is_value())
        .then(|| {
            (
                style.span().start,
                build_time_error(
                    "styled",
                    &readable_code(style),
                    &format!("callback `{name}` refers to itself; recursion is not compiled"),
                ),
            )
        })
}
