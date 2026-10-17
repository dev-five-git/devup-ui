use super::*;
use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[test]
fn commonjs_binding_resolves_factory_when_renderer_is_used() {
    // Given: an actual CommonJS program, retaining its directive.
    let arena = Allocator::default();
    let ast = AstBuilder::new(&arena);
    let mut program = Parser::new(&arena, "'use strict';", SourceType::cjs())
        .parse()
        .program;
    let mut helper = StyledJsx::reserve(&program)
        .unwrap_or_else(|| panic!("CommonJS source must reserve a styled JSX helper"));
    let render = helper.render(
        &ast,
        Expression::new_string_literal(SPAN, "article", None, &ast),
        (
            Expression::new_identifier(SPAN, "className", &ast),
            Expression::new_identifier(SPAN, "style", &ast),
            Expression::new_string_literal(SPAN, "nav", None, &ast),
        ),
    );
    // When: the real binding and renderer execute against a CommonJS factory export.
    program.body.push(
        helper
            .binding(&ast, true)
            .unwrap_or_else(|| panic!("used renderer must produce a CommonJS binding")),
    );
    let binding = Codegen::new().build(&program).code;
    let render = crate::utils::readable_code(&render);
    let script = format!(
        "{binding}\nfunction require(path) {{ if (path !== 'react/jsx-runtime') throw Error(path); return {{ jsx: (target, props) => ({{target, props}}) }}; }}\nconst rest = {{children: 'child'}}; const className = 'caller'; const style = {{color: 'olive'}}; const result = {render}; JSON.stringify(result);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("CommonJS renderer evaluation failed: {error}"));
    // Then: the target and forwarded props come from the imported factory.
    assert!(binding.starts_with("\"use strict\";\nconst "));
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("CommonJS renderer JSON result must be a string"))
            .to_std_string_escaped(),
        r#"{"target":"article","props":{"children":"child","className":"caller","style":{"color":"olive"},"as":"nav"}}"#,
    );
}

#[rstest]
#[case(SourceType::tsx())]
#[case(SourceType::jsx())]
fn helper_is_unreserved_when_source_accepts_jsx(#[case] source_type: SourceType) {
    // Given: a source dialect that accepts JSX.
    let arena = Allocator::default();
    let program = Parser::new(&arena, "", source_type).parse().program;
    // When: helper reservation runs.
    let helper = StyledJsx::reserve(&program);
    // Then: the existing JSX path has no helper state.
    assert!(helper.is_none());
}

#[rstest]
#[case(false)]
#[case(true)]
fn binding_is_absent_when_renderer_is_unused(#[case] commonjs: bool) {
    // Given: a non-JSX module with no renderer compiled.
    let arena = Allocator::default();
    let ast = AstBuilder::new(&arena);
    let program = Parser::new(&arena, "", SourceType::ts()).parse().program;
    let helper = StyledJsx::reserve(&program)
        .unwrap_or_else(|| panic!("non-JSX TypeScript source must reserve a styled JSX helper"));
    // When: module binding insertion is requested.
    let binding = helper.binding(&ast, commonjs);
    // Then: neither module mode receives an unused factory.
    assert!(binding.is_none());
}
