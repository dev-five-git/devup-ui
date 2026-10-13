use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{ExpressionStatement, Program, Statement},
};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SPAN, SourceType};
use oxc_transformer::{JsxRuntime, TransformOptions, Transformer};

pub(super) fn lower(expression: &str, pragma: &str) -> Result<Option<String>, String> {
    let allocator = Allocator::default();
    let source = format!("({expression})");
    let expression = Parser::new(&allocator, &source, SourceType::tsx())
        .parse_expression()
        .map_err(|errors| {
            format!("Cannot compile component: {errors:?}. Fix: correct its expression")
        })?;
    let ast = oxc_ast::builder::AstBuilder::new(&allocator);
    let mut program = Program::new(
        SPAN,
        SourceType::tsx(),
        &source,
        oxc_allocator::Vec::new_in(&ast),
        None,
        oxc_allocator::Vec::new_in(&ast),
        oxc_allocator::Vec::from_iter_in(
            [Statement::ExpressionStatement(ExpressionStatement::boxed(
                SPAN, expression, &ast,
            ))],
            &ast,
        ),
        &ast,
    );
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;
    if !semantic.nodes().iter().any(|node| {
        matches!(
            node.kind(),
            AstKind::JSXElement(_) | AstKind::JSXFragment(_)
        )
    }) {
        return Ok(None);
    }
    let mut options = TransformOptions::default();
    options.jsx.runtime = JsxRuntime::Classic;
    options.jsx.pragma = Some(pragma.to_string());
    let result = Transformer::new(&allocator, std::path::Path::new("component.tsx"), &options)
        .build_with_scoping(semantic.into_scoping(), &mut program);
    if let Some(error) = result.diagnostics.first() {
        return Err(format!(
            "Cannot compile component: {error}. Fix: correct its expression"
        ));
    }
    // Classic JSX transforms the expression in place; no import or binding container exists.
    Ok(Some(
        Codegen::new()
            .build(&program)
            .code
            .trim_end_matches([';', '\n'])
            .to_string(),
    ))
}
