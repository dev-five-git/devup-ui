use super::*;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::operator::LogicalOperator;
use rstest::rstest;

#[rstest]
#[case("()=>slot", "callback()")]
#[case("function(){return slot;}", "callback()")]
#[case("class{static read(){return slot;}}", "callback.read()")]
fn deferred_capture_reads_keep_a_per_evaluation_binding(#[case] body: &str, #[case] probe: &str) {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "closure.tsx", "@devup-ui/react", vec![], None);
    visitor.stylex_captures.push_frame();
    let parse = |source| {
        Parser::new(&arena, source, SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"))
    };
    let expression = visitor.scoped_values(vec![("slot".into(), parse("index++"))], parse(body));
    assert_eq!(visitor.stylex_captures.frames[0].1, [] as [String; 0]);
    let code = crate::utils::readable_code(&expression);
    let script = format!(
        "let index=0;const callbacks=[];for(let count=0;count<3;count++)callbacks.push({code});JSON.stringify(callbacks.map(callback=>{probe}));"
    );
    let actual = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    assert_eq!(
        actual
            .as_string()
            .and_then(|value| value.to_std_string().ok()),
        Some("[0,1,2]".into())
    );
}

#[test]
fn opaque_generated_expressions_keep_their_private_capture_scope() {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "opaque.tsx", "@devup-ui/react", vec![], None);
    visitor.stylex_captures.push_frame();
    let value = Parser::new(&arena, "read()", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let body = Expression::new_identifier(SPAN, "unparsed(() => slot)", &visitor.ast);
    let expression = visitor.scoped_values(vec![("slot".into(), value)], body);
    assert!(matches!(expression, Expression::CallExpression(_)));
    assert_eq!(visitor.stylex_captures.frames[0].1, [] as [String; 0]);
}

#[rstest]
#[case("``", [true, false, true])]
#[case("`nonempty`", [false, true, true])]
#[case("null", [true, false, false])]
#[case("false", [true, false, true])]
#[case("0", [true, false, true])]
fn literal_selection_distinguishes_truthiness_from_nullishness(
    #[case] source: &str,
    #[case] left: [bool; 3],
) {
    let arena = Allocator::default();
    let visitor = DevupVisitor::new(&arena, "literal.tsx", "@devup-ui/react", vec![], None);
    let value = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    for (operator, expected) in [
        LogicalOperator::And,
        LogicalOperator::Or,
        LogicalOperator::Coalesce,
    ]
    .into_iter()
    .zip(left)
    {
        assert_eq!(
            literal_choice(&visitor.bindings, &value, operator),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn non_scalar_cooked_template_tail_is_not_rewritten_as_utf8() {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "tail.tsx", "@devup-ui/react", vec![], None);
    let mut value = Parser::new(&arena, r"`${read()}\uD800`", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::TemplateLiteral(template) = &value else {
        panic!("expected template")
    };
    assert!(
        template.quasis[1]
            .value
            .cooked
            .as_ref()
            .is_some_and(|cooked| cooked.as_str().is_none())
    );
    let before = crate::utils::readable_code(&value);
    let mut captured = Vec::new();
    visitor.capture_leaf(&mut value, &mut captured);
    assert_eq!(captured.len(), 1);
    assert_eq!(crate::utils::readable_code(&captured[0].1), before);
    assert!(
        matches!(&value, Expression::Identifier(read) if read.name.as_str() == captured[0].0.as_str())
    );
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}

#[test]
fn external_tag_keeps_its_own_interpolation_and_capture_boundary() {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "tag.tsx", "@devup-ui/react", vec![], None);
    let mut value = Parser::new(&arena, "externalTag`${read()}`", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let before = crate::utils::readable_code(&value);
    let mut captured = Vec::new();
    assert!(visitor.capture_css_boundary(&mut value, &mut captured));
    assert!(captured.is_empty());
    assert_eq!(crate::utils::readable_code(&value), before);
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}

#[rstest]
#[case(false)]
#[case(true)]
fn sequential_render_captures_keep_initializer_dependencies_and_getter_order(#[case] frame: bool) {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "sequence.tsx", "@devup-ui/react", vec![], None);
    if frame {
        visitor.stylex_captures.push_frame();
    }
    let parse = |source| {
        Parser::new(&arena, source, SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"))
    };
    let expression = visitor.scoped_render(
        vec![
            ("first".into(), parse("source.value")),
            ("second".into(), parse("first + source.value")),
        ],
        parse("[first,second,first]"),
    );
    let declarations = if frame {
        format!("let {};", visitor.stylex_captures.frames[0].1.join(","))
    } else {
        String::new()
    };
    let code = crate::utils::readable_code(&expression);
    let script = format!(
        "let count=0;const source={{get value(){{return ++count;}}}};{declarations}JSON.stringify([({code}),count]);"
    );
    let actual = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    assert_eq!(
        actual
            .as_string()
            .unwrap_or_else(|| panic!("value"))
            .to_std_string_escaped(),
        "[[1,3,1],2]",
        "{script}"
    );
}

#[rstest]
#[case("0", "0px")]
#[case("12", "12px")]
#[case("'2rem'", "2rem")]
fn stylex_number_units_read_their_input_once(#[case] value: &str, #[case] expected: &str) {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "unit.tsx", "@devup-ui/react", vec![], None);
    visitor.stylex_captures.push_frame();
    let input = Parser::new(&arena, "source.value", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let expression = visitor.with_number_unit(input, "px");
    let code = crate::utils::readable_code(&expression);
    assert!(!code.contains("=>"), "{code}");
    let names = visitor.stylex_captures.frames[0].1.join(",");
    let script = format!(
        "let {names};let count=0;const source={{get value(){{count++;return {value};}}}};JSON.stringify([({code}),count]);"
    );
    let actual = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    assert_eq!(
        actual
            .as_string()
            .unwrap_or_else(|| panic!("value"))
            .to_std_string_escaped(),
        format!("[\"{expected}\",1]"),
        "{script}"
    );
}
