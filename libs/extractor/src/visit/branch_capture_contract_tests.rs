use super::*;
use crate::composition::{KnownPart, KnownStyles};
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

#[test]
fn branch_rebasing_retains_styles_and_rewrites_only_owned_metadata() {
    let arena = Allocator::default();
    let visitor = DevupVisitor::new(&arena, "branch.tsx", "@devup-ui/react", vec![], None);
    let parse = |source| {
        Parser::new(&arena, source, SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"))
    };
    let slot = visitor.branch_slot("packet", 1);
    let slot_code = crate::utils::readable_code(&slot);
    let mut reads = BranchReads {
        ast: &visitor.ast,
        reads: FxHashMap::from_iter([("owned".into(), slot)]),
    };
    let styles = vec![crate::ExtractStyleValue::Static(ExtractStaticStyle::new(
        "color", "red", 0, None,
    ))];
    let mut aliases = FxHashMap::from_iter([
        ("owned".into(), styles.clone()),
        ("untouched".into(), styles.clone()),
    ]);
    let mut parts = FxHashMap::from_iter([
        (
            10,
            vec![
                KnownPart::Conditional {
                    test: parse("owned"),
                    consequent: vec![KnownStyles::Known(styles.clone())],
                    alternate: Vec::new(),
                },
                KnownPart::Class(parse("owned")),
            ],
        ),
        (20, vec![KnownPart::Class(parse("owned"))]),
    ]);
    reads.rebase_results(&mut [], &mut parts, &[10], &mut aliases);
    assert_eq!(aliases.get("owned"), Some(&styles));
    assert_eq!(aliases.get(&slot_code), Some(&styles));
    assert_eq!(aliases.get("untouched"), Some(&styles));
    let selected = parts
        .get(&10)
        .unwrap_or_else(|| panic!("missing selected metadata"));
    let KnownPart::Conditional {
        test,
        consequent,
        alternate,
    } = &selected[0]
    else {
        panic!("expected condition")
    };
    assert_eq!(crate::utils::readable_code(test), slot_code);
    assert!(matches!(&consequent[0], KnownStyles::Known(values) if values == &styles));
    assert!(alternate.is_empty());
    let KnownPart::Class(class) = &selected[1] else {
        panic!("expected class")
    };
    assert_eq!(crate::utils::readable_code(class), slot_code);
    let KnownPart::Class(control) = &parts.get(&20).unwrap_or_else(|| panic!("missing control"))[0]
    else {
        panic!("expected control class")
    };
    assert_eq!(crate::utils::readable_code(control), "owned");
}

#[rstest]
#[case("css`color:red`", true, true)]
#[case("css(enabled ? {color:'red'} : {color:'blue'})", true, true)]
#[case("css(unknownRules())", false, false)]
#[case("css(externalClass)", true, false)]
#[serial_test::serial]
fn inline_css_controller_keeps_its_proved_shape_boundary(
    #[case] source: &str,
    #[case] known: bool,
    #[case] cached_rules: bool,
) {
    let _state = crate::test_state::TestStateGuard::new();
    let arena = Allocator::default();
    let source = format!("import {{css}} from '@devup-ui/react';{source};");
    let mut parsed = Parser::new(&arena, &source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let scoping = std::rc::Rc::new(
        SemanticBuilder::new()
            .build(&parsed.program)
            .semantic
            .into_scoping(),
    );
    let mut visitor = DevupVisitor::new(&arena, "controller.tsx", "@devup-ui/react", vec![], None);
    visitor.bindings.scope(scoping);
    let symbol = visitor.bindings.imported("css");
    assert!(visitor.bindings.export(symbol, "css"));
    let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.last_mut() else {
        panic!("expected expression")
    };
    let offset = statement.expression.span().start;
    let controller = visitor.css_controller(&mut statement.expression);
    if known {
        assert!(visitor.inline_css_parts.contains_key(&offset));
        assert_eq!(visitor.css_capture_offsets, [offset]);
        assert_eq!(!visitor.css_capture_rules.is_empty(), cached_rules);
        assert!(
            matches!(&statement.expression, Expression::StringLiteral(value) if value.value.is_empty())
        );
        assert!(!matches!(controller, Expression::NullLiteral(_)));
        assert!(!visitor.css_cache_collect);
    } else {
        assert!(matches!(controller, Expression::NullLiteral(_)));
        assert!(visitor.inline_css_parts.is_empty());
        assert_eq!(visitor.css_capture_offsets, [] as [u32; 0]);
        assert!(crate::utils::readable_code(&statement.expression).contains("unknownRules()"));
    }
}

#[rstest]
#[case("`` && {color:read()}", false)]
#[case("`truthy` || {color:read()}", false)]
#[case("null ?? {color:read()}", true)]
fn literal_css_choices_capture_only_the_selected_branch(#[case] source: &str, #[case] reads: bool) {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "literal-css.tsx", "@devup-ui/react", vec![], None);
    let mut value = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut captured = Vec::new();
    visitor.capture_css_logical(&mut value, &mut captured);
    assert_eq!(!captured.is_empty(), reads);
    let retained = crate::utils::readable_code(&value);
    if reads {
        assert_eq!(captured.len(), 1);
        assert_eq!(crate::utils::readable_code(&captured[0].1), "read()");
        assert!(retained.contains("color"));
    } else {
        assert!(!retained.contains("read()"));
    }
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}

#[test]
fn nonliteral_coalesce_controller_uses_nullish_equality() {
    let arena = Allocator::default();
    let mut visitor =
        DevupVisitor::new(&arena, "coalesce-css.tsx", "@devup-ui/react", vec![], None);
    let mut value = Parser::new(&arena, "control ?? {color:read()}", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut captured = Vec::new();
    visitor.capture_css_logical(&mut value, &mut captured);
    assert_eq!(captured.len(), 1);
    let captured_code = crate::utils::readable_code(&captured[0].1);
    assert!(captured_code.contains("== null"), "{captured_code}");
    assert_eq!(captured_code.matches("control").count(), 1);
    assert_eq!(captured_code.matches("read()").count(), 1);
    assert!(matches!(value, Expression::ConditionalExpression(_)));
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}
#[test]
fn coalesce_capture_when_executed_distinguishes_falsy_values_from_null() {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(
        &arena,
        "coalesce-runtime.tsx",
        "@devup-ui/react",
        vec![],
        None,
    );
    let mut value = Parser::new(&arena, "control ?? {color:read()}", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut captured = Vec::new();
    visitor.capture_css_logical(&mut value, &mut captured);
    assert_eq!(captured.len(), 1);
    let code = crate::utils::readable_code(&captured[0].1);
    for (literal, expected) in [
        ("null", "[true,[\"right\"]]"),
        ("undefined", "[true,[\"right\"]]"),
        ("false", "[false,[]]"),
        ("0", "[false,[]]"),
        ("''", "[false,[]]"),
    ] {
        let script = format!(
            "let trace=[];function read(){{trace.push('right');return 'red'}}const control={literal};const packet={code};JSON.stringify([packet[0],trace]);"
        );
        let output = boa_engine::Context::default()
            .eval(boa_engine::Source::from_bytes(&script))
            .unwrap_or_else(|error| panic!("{error}: {script}"));
        let actual = output
            .as_string()
            .unwrap_or_else(|| panic!("expected JSON result"))
            .to_std_string_escaped();
        assert_eq!(actual, expected, "{literal}: {script}");
    }
}

#[rstest]
#[case("`a`", Some(true))]
#[case("``", Some(false))]
#[case("'a'", Some(true))]
#[case("''", Some(false))]
#[case("1", Some(true))]
#[case("0", Some(false))]
#[case("true", Some(true))]
#[case("false", Some(false))]
#[case("[]", Some(true))]
#[case("() => 0", Some(true))]
#[case("a + b", Some(false))]
#[case("-a", Some(false))]
#[case("void a", None)]
#[case("a", None)]
fn literals_tell_whether_they_are_defined_and_truthy(
    #[case] source: &str,
    #[case] expected: Option<bool>,
) {
    let arena = Allocator::default();
    let value = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(defined(&value), expected);
}
