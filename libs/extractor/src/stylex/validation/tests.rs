use super::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

fn with_expression(source: &str, check: impl FnOnce(&Expression<'_>)) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expected expression statement");
    };
    check(statement.expression.without_parentheses());
}

fn resolve(callee: &Expression<'_>) -> Option<StylexFunction> {
    match callee {
        Expression::Identifier(identifier) => {
            StylexFunction::from_export_name(identifier.name.as_str())
        }
        Expression::StaticMemberExpression(member) if matches!(&member.object, Expression::Identifier(identifier) if identifier.name == "sx") => {
            StylexFunction::from_export_name(member.property.name.as_str())
        }
        _ => None,
    }
}

#[rstest]
#[case("  sx.include();", Some("exactly one namespace"))]
#[case(
    "  sx.include(styles.base, styles.other);",
    Some("exactly one namespace")
)]
#[case("  sx.include(...namespaces);", Some("exactly one namespace"))]
#[case("  include(styles.base);", None)]
#[case("  sx.types();", Some("requires a value"))]
#[case("  sx.types.color();", Some("requires a value"))]
#[case("  sx.types.color('red', 'blue');", None)]
#[case("  sx.types.color(...values);", Some("requires a value"))]
#[case("  sx.types.color('red');", None)]
#[case("  types.color({default: 'red', '@media print': 'blue'});", None)]
#[case("  sx.firstThatWorks();", None)]
#[case("  firstThatWorks('red');", None)]
#[case("  sx.firstThatWorks('red', 'blue');", None)]
#[case("  other.types.color();", None)]
#[case("  sx.create();", None)]
#[case("  sx.types.color('red', 42, null, false, {a:['x',true]});", None)]
#[case(
    "  sx.types.color('red', runtime());",
    Some("extra arguments are evaluated")
)]
#[case(
    "  sx.types.color('red', ...values);",
    Some("extra arguments are evaluated")
)]
#[case(
    "  sx.types.color('red', {get a(){return 'x'}});",
    Some("extra arguments are evaluated")
)]
#[case(
    "  sx.types.color('red', {[runtime()]: 'x'});",
    Some("extra arguments are evaluated")
)]
#[case(
    "  sx.types.color('red', {...values});",
    Some("extra arguments are evaluated")
)]
#[case(
    "  sx.types.color('red', [runtime]);",
    Some("extra arguments are evaluated")
)]
fn helper_arity_when_parsed(#[case] source: &str, #[case] cause: Option<&str>) {
    with_expression(source, |expression| {
        let Expression::CallExpression(call) = expression else {
            panic!("expected call")
        };
        let result = validate_helper_call(call, &resolve);
        match cause {
            Some(cause) => {
                let Err((offset, message)) = result else {
                    panic!("expected boundary error")
                };
                assert_eq!(offset, 2);
                assert!(message.contains(cause));
                assert!(message.contains(&readable_code(&call.callee)));
            }
            None => assert_eq!(result, Ok(())),
        }
    });
}

#[rstest]
#[case("  sx.firstThatWorks('red');", Some("stylex.create()"))]
#[case("  include(styles.base);", Some("namespace spread"))]
#[case("  sx.types.color('red');", Some("stylex.createTheme()"))]
#[case("  types.color(null);", Some("consumed value"))]
#[case("  sx.types(null);", Some("consumed value"))]
#[case("  sx.props(styles.base);", None)]
#[case("  other.include(styles.base);", None)]
fn context_error_when_helper_survives(#[case] source: &str, #[case] fix: Option<&str>) {
    with_expression(source, |expression| {
        let Expression::CallExpression(call) = expression else {
            panic!("expected call")
        };
        let result = unconsumed_helper_error(call, &resolve);
        match fix {
            Some(fix) => {
                let Some((offset, message)) = result else {
                    panic!("expected context error")
                };
                assert_eq!(offset, 2);
                assert!(message.contains(fix));
                assert!(message.contains(&readable_code(&call.callee)));
            }
            None => assert_eq!(result, None),
        }
    });
}

#[rstest]
#[case("@media print", true)]
#[case("@supports(display: grid)", true)]
#[case("@container sidebar (width > 1px)", true)]
#[case("@mediaOops print", false)]
#[case("@supportsOops (display: grid)", false)]
#[case("@containerOops sidebar", false)]
#[case("@media", false)]
#[case("@supports   ", false)]
#[case("@container", false)]
#[case("@unknown print", false)]
#[case("default", true)]
#[case(":hover", true)]
fn condition_key_when_parsed(#[case] key: &str, #[case] valid: bool) {
    let source = format!("({{ '{key}': 'red' }})");
    with_expression(&source, |expression| {
        let Expression::ObjectExpression(object) = expression else {
            panic!("expected object")
        };
        let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
            panic!("expected property")
        };
        let result = validate_at_rule_condition(&property.key, "stylex.create");
        assert_eq!(result.is_ok(), valid);
        if let Err((offset, message)) = result {
            assert_eq!(offset, 3);
            assert!(message.contains(key));
            assert!(message.contains("nonempty query"));
        }
    });
}

#[test]
fn condition_key_when_runtime_computed() {
    with_expression("({ [runtime()]: 'red' })", |expression| {
        let Expression::ObjectExpression(object) = expression else {
            panic!("expected object")
        };
        let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
            panic!("expected property")
        };
        let result = validate_at_rule_condition(&property.key, "stylex.defineVars");
        assert_eq!(result, Err(key_error("stylex.defineVars", &property.key)));
    });
}

#[rstest]
#[case("null", true)]
#[case("'--text'", true)]
#[case("(null as const)", true)]
#[case("runtimeValue", false)]
#[case("getValue()", false)]
#[case("42", false)]
#[case("false", false)]
#[case("({ text: null })", false)]
#[case("({ default: 'red', '@media print': 'blue' })", false)]
fn contract_placeholder_when_parsed(#[case] value: &str, #[case] valid: bool) {
    let source = format!("  ({value});");
    with_expression(&source, |expression| {
        let result = validate_contract_placeholder("palette.text", expression);
        assert_eq!(result.is_ok(), valid);
        if let Err((offset, message)) = result {
            assert_eq!(offset, expression.span().start);
            assert!(message.contains("palette.text"));
            assert!(message.contains("paletteText"));
            assert!(message.contains(&readable_code(expression)));
        }
    });
}
