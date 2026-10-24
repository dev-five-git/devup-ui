use super::*;
use crate::extract_style::extract_dynamic_style::ExtractDynamicStyle;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case(true)]
#[case(false)]
fn dynamic_target_when_only_one_shorthand_axis_wins_keeps_only_that_axis(#[case] allowed: bool) {
    let mut style = ExtractStyleProp::Static(ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
        "padding-left",
        0,
        "read()",
        None,
    )));
    let targets = FxHashSet::from_iter([if allowed {
        "padding-left"
    } else {
        "padding-right"
    }
    .to_string()]);

    let retained = retains_target(&mut style, &targets);

    assert_eq!(retained, allowed);
    assert!(
        matches!(style, ExtractStyleProp::Static(ExtractStyleValue::Dynamic(value))
        if value.property() == "padding-left" && value.identifier() == "read()")
    );
}

#[rstest]
#[case(&["padding-left"][..], (true, false, true))]
#[case(&["padding-right"][..], (false, true, true))]
#[case(&[][..], (false, false, false))]
#[case(&["padding-left", "padding-right"][..], (true, true, true))]
fn conditional_target_when_a_later_axis_wins_prunes_each_branch_independently(
    #[case] allowed: &[&str],
    #[case] expected: (bool, bool, bool),
) {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut style = ExtractStyleProp::Conditional {
        condition: Expression::new_boolean_literal(SPAN, true, &ast),
        consequent: Some(Box::new(ExtractStyleProp::StaticArray(vec![
            ExtractStyleProp::Static(ExtractStyleValue::Static(ExtractStaticStyle::new(
                "padding-left",
                "4px",
                0,
                None,
            ))),
        ]))),
        alternate: Some(Box::new(ExtractStyleProp::Static(
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("padding-right", 0, "size", None)),
        ))),
    };
    let targets = allowed.iter().map(|name| (*name).to_string()).collect();

    let retained = retains_target(&mut style, &targets);

    assert_eq!(retained, expected.2);
    let ExtractStyleProp::Conditional {
        consequent,
        alternate,
        ..
    } = style
    else {
        panic!("conditional required")
    };
    assert_eq!(
        (consequent.is_some(), alternate.is_some()),
        (expected.0, expected.1)
    );
    if let Some(branch) = consequent {
        assert!(
            matches!(branch.extract().as_slice(), [ExtractStyleValue::Static(value)]
            if value.property() == "padding-left" && value.value() == "4px")
        );
    }
    if let Some(branch) = alternate {
        assert!(
            matches!(branch.extract().as_slice(), [ExtractStyleValue::Dynamic(value)]
            if value.property() == "padding-right" && value.identifier() == "size")
        );
    }
}

#[test]
fn unreadable_target_when_pruning_other_axes_preserves_the_diagnostic() {
    let mut style = ExtractStyleProp::Unreadable {
        offset: 17,
        code: "bad()".to_string(),
        prop: false,
        requirement: Some("literal"),
    };

    let retained = retains_target(&mut style, &FxHashSet::default());

    assert!(retained);
    assert!(
        matches!(style, ExtractStyleProp::Unreadable { offset:17, code, prop:false, requirement:Some("literal") } if code == "bad()")
    );
}

#[test]
fn authored_props_when_a_computed_dom_key_is_unknown_keep_its_evaluation() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut expression = Parser::new(
        &allocator,
        "({[key()]:value(),color:'red',id:'kept'})",
        SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::ObjectExpression(object) =
        crate::utils::unwrap_syntax_only_mut(&mut expression)
    else {
        panic!("object required")
    };

    let result = extract_props(&ast, object, &mut FxHashSet::default(), &mut |_| {});

    let values: Vec<_> = result
        .styles
        .iter()
        .flat_map(ExtractStyleProp::extract)
        .collect();
    assert!(
        matches!(values.as_slice(), [ExtractStyleValue::Static(value)]
        if value.property() == "color" && value.value() == "red")
    );
    let code = crate::utils::expression_to_code(&expression);
    let script = format!(
        "let trace=[];const key=()=>{{trace.push('key');return 'data-name'}};const value=()=>{{trace.push('value');return 'raw'}};const props={code};JSON.stringify([trace,props]);"
    );
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    assert_eq!(
        value
            .as_string()
            .unwrap_or_else(|| panic!("string required: {value:?}"))
            .to_std_string_escaped(),
        r#"[["key","value"],{"data-name":"raw","id":"kept"}]"#
    );
}

#[rstest]
#[case("({color:'red',...{bg:'blue',...{id:'dom',p:2}}})", &["color", "bg", "p"])]
#[case("({color:'red',...rest})", &[])]
#[case("rest", &[])]
fn written_keys_when_a_spread_is_nested_require_the_entire_shape_to_be_known(
    #[case] source: &str,
    #[case] expected: &[&str],
) {
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));

    let keys = super::super::order::written_properties(&expression);

    assert_eq!(keys, expected);
}

#[test]
fn flattening_when_known_spreads_are_nested_preserves_effect_order_and_opaque_spreads() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut expression = Parser::new(
        &allocator,
        "({...{color:read('a','red'),...{id:read('b','kept')}},...rest,title:read('c','last')})",
        SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::ObjectExpression(object) =
        crate::utils::unwrap_syntax_only_mut(&mut expression)
    else {
        panic!("object required")
    };

    flatten_known(&ast, object);

    assert_eq!(object.properties.len(), 4);
    assert_eq!(
        object
            .properties
            .iter()
            .filter(|property| matches!(property, ObjectPropertyKind::SpreadProperty(_)))
            .count(),
        1
    );
    assert!(object.properties.iter().all(|property| match property {
        ObjectPropertyKind::SpreadProperty(spread) => is_unknown_spread(&spread.argument),
        ObjectPropertyKind::ObjectProperty(_) => true,
    }));
    let code = crate::utils::expression_to_code(&expression);
    let script = format!(
        "let trace=[];const read=(key,value)=>{{trace.push(key);return value}};const rest={{get other(){{return read('spread','raw')}}}};const props={code};JSON.stringify([trace,props]);"
    );
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    assert_eq!(
        value
            .as_string()
            .unwrap_or_else(|| panic!("string required: {value:?}"))
            .to_std_string_escaped(),
        r#"[["a","b","spread","c"],{"color":"red","id":"kept","other":"raw","title":"last"}]"#
    );
}
