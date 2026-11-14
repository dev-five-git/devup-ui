use super::*;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("false ?? 'external'", None)]
#[case("false || 'external'", Some("external"))]
#[case("null ?? 'external'", Some("external"))]
fn composition_when_an_empty_left_value_has_different_logical_meanings(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    let visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    let mut parts = Vec::new();
    assert_eq!(
        visitor.known_parts(&statement.expression, &mut parts, Text::Classes),
        Some(())
    );
    let classes: Vec<_> = parts
        .iter()
        .filter_map(|part| match part {
            KnownPart::Class(Expression::StringLiteral(value)) => Some(value.value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(classes, expected.map(Some).into_iter().collect::<Vec<_>>());
}

#[rstest]
#[case("({color:'red',id:'keep',[key]:'computed',...rest})", true)]
#[case("({...{color:'red',id:'keep'}})", false)]
#[case("({get color(){return 'red'}})", true)]
fn spread_shape_when_classified_uses_the_whole_object(#[case] source: &str, #[case] unknown: bool) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    assert_eq!(
        element_props::is_unknown_spread(&statement.expression),
        unknown
    );
}

#[rstest]
#[case("<Box styleOrder=\"bad\"/>", "\"bad\"", false)]
#[case("<Box styleOrder=<span/>/>", "<span />", false)]
#[case("<Box styleOrder=<>bad</>/>", "<>bad</>", false)]
#[case("<Box styleOrder={runtime()}/>", "runtime()", true)]
fn order_diagnostics_preserve_authored_value_and_offset(
    #[case] source: &str,
    #[case] code: &str,
    #[case] dynamic: bool,
) {
    let allocator = Allocator::default();
    let mut visitor = DevupVisitor::new(&allocator, "order.tsx", "@devup-ui/react", vec![], None);
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::JSXElement(element) = expression else {
        panic!("expected JSX element")
    };
    let JSXAttributeItem::Attribute(attribute) = &element.opening_element.attributes[0] else {
        panic!("expected styleOrder attribute")
    };
    let value = attribute
        .value
        .as_ref()
        .unwrap_or_else(|| panic!("expected explicit value"));
    let parsed = visitor.style_order_attribute("Box", value);
    assert!(if dynamic {
        matches!(parsed, ParsedStyleOrder::Unsupported)
    } else {
        matches!(parsed, ParsedStyleOrder::None)
    });
    let expected = if dynamic {
        crate::utils::element_error("Box", code, crate::style_order::STYLE_ORDER_VALUE)
    } else {
        crate::style_order::invalid_order(code)
    };
    let offset = match value {
        JSXAttributeValue::ExpressionContainer(container) => {
            container
                .expression
                .as_expression()
                .unwrap_or_else(|| panic!("expected expression"))
                .span()
                .start
        }
        JSXAttributeValue::StringLiteral(literal) => literal.span.start,
        JSXAttributeValue::Element(element) => element.span.start,
        JSXAttributeValue::Fragment(fragment) => fragment.span.start,
    };
    assert_eq!(visitor.errors, vec![(offset, expected)]);
}

#[test]
fn canonical_numeric_order_string_remains_a_static_layer() {
    let allocator = Allocator::default();
    let mut visitor = DevupVisitor::new(&allocator, "order.tsx", "@devup-ui/react", vec![], None);
    let expression = Parser::new(&allocator, "<Box styleOrder=\"2\"/>", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::JSXElement(element) = expression else {
        panic!("expected JSX element")
    };
    let JSXAttributeItem::Attribute(attribute) = &element.opening_element.attributes[0] else {
        panic!("expected styleOrder attribute")
    };
    let value = attribute
        .value
        .as_ref()
        .unwrap_or_else(|| panic!("expected explicit value"));
    assert!(matches!(
        visitor.style_order_attribute("Box", value),
        ParsedStyleOrder::Static(2)
    ));
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}
#[test]
fn local_object_argument_reports_the_authored_argument_location() {
    let allocator = Allocator::default();
    let source = concat!("function f(){const local={", "color:unknown};css(local);}");
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty());
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let local = scoping
        .symbol_ids()
        .find(|symbol| scoping.symbol_name(*symbol) == "local")
        .unwrap_or_else(|| panic!("expected local object binding"));
    let mut visitor = DevupVisitor::new(&allocator, "local.tsx", "@devup-ui/react", vec![], None);
    let scoping = std::rc::Rc::new(scoping);
    visitor.bindings.scope(std::rc::Rc::clone(&scoping));
    visitor.style_values = crate::style_values::StyleValues::new(scoping);
    visitor.local_objects.insert(local);
    let Statement::FunctionDeclaration(function) = &parsed.program.body[0] else {
        panic!("expected function")
    };
    let body = function
        .body
        .as_ref()
        .unwrap_or_else(|| panic!("expected body"));
    let Statement::ExpressionStatement(statement) = &body.statements[1] else {
        panic!("expected call statement")
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("expected call")
    };
    assert_eq!(
        visitor
            .style_values
            .symbol(call.arguments[0].to_expression()),
        Some(local)
    );
    let before = crate::utils::readable_code(&statement.expression);
    visitor.local_arguments("css", &call.arguments);
    let expected_offset = u32::try_from(
        source
            .find("local)")
            .unwrap_or_else(|| panic!("missing authored argument")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        visitor.errors,
        vec![(
            expected_offset,
            crate::utils::build_time_error("css", "local", crate::utils::LOCAL_STYLES)
        )]
    );
    assert_eq!(crate::utils::readable_code(&statement.expression), before);
}
