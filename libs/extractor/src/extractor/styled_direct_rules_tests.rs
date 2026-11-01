use super::*;
use oxc_ast::ast::{ObjectPropertyKind, Statement};

#[test]
fn discarded_rules_when_cloned_preserve_the_original_semantic_reference() {
    // Given: a real module binding and the object expression that reads it.
    let arena = oxc_allocator::Allocator::default();
    let builder = AstBuilder::new(&arena);
    let parsed = oxc_parser::Parser::new(
        &arena,
        "const tone='red'; ({ color: tone });",
        oxc_span::SourceType::mjs(),
    )
    .parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let symbol = scoping
        .symbol_ids()
        .find(|id| scoping.symbol_name(*id) == "tone")
        .unwrap_or_else(|| panic!("expected tone binding"));
    let reference = scoping.get_resolved_reference_ids(symbol)[0];
    let Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
        panic!("expected rules")
    };
    let direct = Direct {
        builder: &builder,
        name: "captures",
        scope: statement.expression.clone_in_with_semantic_ids(&arena),
        next: 1,
    };
    // When: the direct-capture contract retains an object for evaluation only.
    let Ok(cloned) = direct.discarded_object(&statement.expression) else {
        panic!("direct discarded rules must be retained")
    };
    // Then: the clone still resolves to the original binding, rather than a new free name.
    let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(&cloned) else {
        panic!("expected cloned object")
    };
    let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        panic!("expected color property")
    };
    let Expression::Identifier(identifier) = &property.value else {
        panic!("expected original reference")
    };
    assert_eq!(identifier.reference_id.get(), Some(reference));
    assert_eq!(scoping.get_reference(reference).symbol_id(), Some(symbol));
    let script = format!(
        "const tone='red';JSON.stringify({});",
        crate::utils::readable_code(&cloned)
    );
    let observed = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected JSON observation"))
        .to_std_string_escaped();
    assert_eq!(observed, r#"{"color":"red"}"#);
}
