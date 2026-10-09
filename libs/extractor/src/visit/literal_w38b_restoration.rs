use super::*;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

mod f195;
mod finite_origins;

fn local<'a>(allocator: &'a Allocator, source: &'a str) -> (DevupVisitor<'a>, Expression<'a>) {
    let aliased = crate::import_alias_visit::transform_import_aliases_with_edits(
        source,
        "a.tsx",
        "@devup-ui/react",
        &std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        )]),
    );
    let source = allocator.alloc_str(&aliased.code);
    let mut parsed = oxc_parser::Parser::new(allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = oxc_semantic::SemanticBuilder::new().build(&parsed.program);
    assert_eq!(semantic.diagnostics.len(), 0);
    let mut visitor = DevupVisitor::new(allocator, "a.tsx", "@devup-ui/react", vec![], None);
    visitor.source = Some(source);
    visitor.reuse_scoping(Some(Rc::new(semantic.semantic.into_scoping())));
    let scoping = visitor
        .scoping_of(&parsed.program)
        .unwrap_or_else(|| panic!("real parsed scoping"));
    visitor.names.reserve(&scoping);
    visitor.bindings.scope(Rc::clone(&scoping));
    visitor.style_values.scope(scoping);
    let statement = parsed
        .program
        .body
        .pop()
        .unwrap_or_else(|| panic!("ClassNames expression"));
    for statement in &mut parsed.program.body {
        visitor.visit_statement(statement);
    }
    let Statement::ExpressionStatement(statement) = statement else {
        panic!("ClassNames expression statement required")
    };
    let Expression::JSXElement(mut element) = statement.unbox().expression else {
        panic!("ClassNames element required")
    };
    let JSXElementName::IdentifierReference(name) = &element.opening_element.name else {
        panic!("ClassNames binding required")
    };
    assert!(visitor.bindings.is_class_names(name));
    let JSXChild::ExpressionContainer(container) = &mut element.children[0] else {
        panic!("actual child function required")
    };
    let function = container
        .expression
        .as_expression_mut()
        .unwrap_or_else(|| panic!("child expression"));
    let (params, body) =
        render_function(function).unwrap_or_else(|| panic!("real render function"));
    let names = class_names_params(params).unwrap_or_else(|| panic!("real child names"));
    let symbols = ClassNamesSymbols::of(params);
    let mut rendered = body.take_in(&visitor.ast);
    let mut calls = ClassNamesCalls {
        ast: &visitor.ast,
        bindings: &visitor.bindings,
        source: visitor.source,
        names,
        symbols,
        unread: None,
    };
    calls.visit_expression(&mut rendered);
    assert!(calls.unread.is_none());
    visitor.class_names_scope.push(symbols);
    (visitor, rendered)
}

fn evaluated(value: &Expression<'_>, setup: &str) -> serde_json::Value {
    let script = format!(
        "const trace=[];{setup};JSON.stringify([({}).trim().replace(/\\s+/g,' '),trace])",
        readable_code(value)
    );
    let mut context = boa_engine::Context::default();
    let result = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    let json = result
        .to_string(&mut context)
        .unwrap_or_else(|error| panic!("JSON string: {error}"))
        .to_std_string_escaped();
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("evaluation JSON: {error}"))
}

#[rstest]
#[case(true, 2)]
#[case(false, 3)]
#[serial]
fn local_raw_template_when_order_is_finite_captures_authored_getter_once(
    #[case] active: bool,
    #[case] order: u8,
) {
    // Given: bindings and the raw argument come from the actual ClassNames child.
    let source = "import {ClassNames} from '@emotion/react';<ClassNames>{({css,cx})=>css(`style-order:${state.active?2:3};color:red`)}</ClassNames>;";
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, source);
    let span = value.span();
    let Expression::CallExpression(call) = &value else {
        panic!("raw local call required")
    };
    assert!(matches!(&call.arguments[0], Argument::TemplateLiteral(_)));
    let Argument::TemplateLiteral(template) = &call.arguments[0] else {
        panic!("raw template required")
    };
    let Expression::ConditionalExpression(hole) = &template.expressions[0] else {
        panic!("real conditional hole required")
    };
    let getter_span = hole.test.span();
    assert_eq!(
        visitor.known_parts(
            call.arguments[0].to_expression(),
            &mut Vec::new(),
            Text::Rules
        ),
        Some(())
    );
    css::debug::set_debug(true);
    // When: the real local call compiler captures the supported raw hole.
    assert!(visitor.compile_class_names_call(&mut value));
    css::debug::set_debug(false);
    // Then: the original call span and one selected order survive with one getter read.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    assert!(
        matches!(&value, Expression::CallExpression(_)),
        "{}",
        readable_code(&value)
    );
    let Expression::CallExpression(captured) = &value else {
        panic!("capture wrapper required")
    };
    assert!(
        captured
            .arguments
            .iter()
            .any(|argument| argument.span() == getter_span
                && readable_argument(argument) == "state.active")
    );
    assert_eq!(
        evaluated(
            &value,
            &format!("const state={{get active(){{trace.push('order');return {active};}}}}")
        ),
        serde_json::json!([format!("color-0-red--{order}"), ["order"]])
    );
    assert!(visitor.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.value()=="red" && style.style_order()==Some(order))));
}

#[rstest]
#[case(
    "cx('external-red',`external-blue`)",
    "external-red external-blue",
    "template"
)]
#[case("cx(true?'external-red':'external-blue')", "external-red", "template")]
#[case("cx(true?'external-red':null)", "external-red", "conditional")]
#[case("cx('external-red'||'external-blue')", "external-red", "string")]
#[serial]
fn local_classes_when_capture_free_restore_final_source_span(
    #[case] call: &str,
    #[case] expected: &str,
    #[case] shape: &str,
) {
    // Given: a capture-free, source-bound external class composition.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>{call}}}</ClassNames>;"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    // When: the local call runs through preparation, composition and capture.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: the value is retained and the actual final root is observable.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(evaluated(&value, ""), serde_json::json!([expected, []]));
    assert_eq!(root(&value), shape, "{}", readable_code(&value));
    println!(
        "local source={call}; original={span:?}; final={:?}; code={}",
        value.span(),
        readable_code(&value)
    );
    assert_eq!(value.span(), span);
}

fn root(value: &Expression<'_>) -> &'static str {
    match value {
        Expression::StringLiteral(_) => "string",
        Expression::TemplateLiteral(_) => "template",
        Expression::CallExpression(_) => "call",
        Expression::ConditionalExpression(_) => "conditional",
        Expression::LogicalExpression(_) => "logical",
        Expression::Identifier(_) => "identifier",
        Expression::ComputedMemberExpression(_) => "member",
        _ => "other",
    }
}
