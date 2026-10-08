use super::*;

#[rstest]
#[case("flag?'same':'same';", (true, r#"["same",[]]"#))]
#[case("flag?`raw\\n${external}`:`raw\\n${external}`;", (true, r#"["raw\nleaf",[]]"#))]
#[case("flag?external:external;", (true, r#"["leaf",[]]"#))]
#[case("flag?(external||take()):(external||take());", (false, r#"["leaf",[]]"#))]
#[case("flag?`${external||take()}`:`${external||take()}`;", (false, r#"["leaf",[]]"#))]
#[serial]
fn generic_conditional_when_branches_look_equal_preserves_original_equality(
    #[case] source: &str,
    #[case] contract: (bool, &str),
    #[values(true, false)] flag: bool,
) {
    // Given: a parsed conditional supplies both actual branch payloads independently.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let Expression::ConditionalExpression(condition) = parsed(&allocator, source) else {
        panic!("parsed conditional required")
    };
    let winning_span = condition.consequent.span();
    let winning_code = readable_code(&condition.consequent);
    let mut props = vec![ExtractStyleProp::Conditional {
        condition: condition.test.clone_in(&allocator),
        consequent: Some(Box::new(ExtractStyleProp::Expression {
            styles: vec![],
            expression: condition.consequent.clone_in(&allocator),
        })),
        alternate: Some(Box::new(ExtractStyleProp::Expression {
            styles: vec![],
            expression: condition.alternate.clone_in(&allocator),
        })),
    }];
    // When: unchanged default generation applies its existing equality predicate.
    let actual = gen_class_names(&ast, &mut props, None, None)
        .unwrap_or_else(|| panic!("conditional class"));
    let raw_code = readable_code(&actual);
    println!(
        "W38H_F195_RAW {}",
        serde_json::json!({
            "schema": 1,
            "kind": "equality",
            "source": source,
            "mode": "generator",
            "flag": flag,
            "code": raw_code,
            "code_utf8_bytes": raw_code.len(),
            "span": [actual.span().start, actual.span().end],
            "stylesheet": null,
        })
    );
    // Then: equal supported operands retain the winner; Logical descendants do not collapse.
    assert_eq!(
        !matches!(actual, Expression::ConditionalExpression(_)),
        contract.0
    );
    if contract.0 {
        match (&actual, &condition.consequent) {
            (Expression::StringLiteral(_), Expression::StringLiteral(_)) => {
                assert_eq!(actual.span(), SPAN);
            }
            (Expression::TemplateLiteral(actual), Expression::TemplateLiteral(expected)) => {
                assert_eq!(actual.span, winning_span);
                assert_eq!(
                    format!("{:?}", actual.quasis),
                    format!("{:?}", expected.quasis)
                );
            }
            (Expression::Identifier(actual), Expression::Identifier(_)) => {
                assert_eq!(actual.span, winning_span);
            }
            _ => panic!("unexpected equal-branch operands"),
        }
        assert_eq!(readable_code(&actual), winning_code);
    }
    let setup = format!(
        "const flag={flag};const external='leaf';function take(){{trace.push('fallback');return 'fallback';}}"
    );
    let expected: serde_json::Value =
        serde_json::from_str(contract.1).unwrap_or_else(|error| panic!("fixture JSON: {error}"));
    assert_eq!(json_evaluated(&actual, &setup), expected);
}

#[rstest]
#[case("['  alpha  ','   ',' beta '];", Some("alpha beta"))]
#[case("['   ','\\n'];", None)]
#[serial]
fn merge_when_only_strings_are_supplied_trims_and_omits_empty_fragments(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) {
    // Given: every string fragment comes from a parsed array, including whitespace-only input.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let Expression::ArrayExpression(array) = parsed(&allocator, source) else {
        panic!("parsed fragments required")
    };
    let fragments = array.elements.iter().map(|element| {
        element
            .as_expression()
            .unwrap_or_else(|| panic!("array expression"))
            .clone_in(&allocator)
    });
    // When: the original merger consumes the source fragments.
    let actual = merge_expression_for_class_name(&ast, fragments);
    let raw_code = actual.as_ref().map(readable_code);
    println!(
        "W38H_F195_RAW {}",
        serde_json::json!({
            "schema": 1,
            "kind": "static_fragments",
            "source": source,
            "mode": "merger",
            "code": raw_code,
            "code_utf8_bytes": raw_code.as_ref().map(String::len),
            "stylesheet": null,
        })
    );
    // Then: static fragments have one separator, or produce no class at all.
    match (actual, expected) {
        (Some(actual), Some(expected)) => {
            assert_eq!(
                json_evaluated(&actual, ""),
                serde_json::json!([expected, []])
            );
        }
        (None, None) => {}
        (actual, expected) => panic!("unexpected merge {actual:?}, expected {expected:?}"),
    }
}

#[test]
#[serial]
fn merge_when_static_and_two_dynamic_fragments_are_supplied_preserves_exact_spacing() {
    // Given: static fragments and two independently parsed effectful expressions.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = "['  alpha  ',first(),'   ',' beta ',second()];";
    let Expression::ArrayExpression(array) = parsed(&allocator, source) else {
        panic!("parsed fragments required")
    };
    let fragments = array.elements.iter().map(|element| {
        element
            .as_expression()
            .unwrap_or_else(|| panic!("array expression"))
            .clone_in(&allocator)
    });
    // When: the original merger constructs the combined template.
    let actual =
        merge_expression_for_class_name(&ast, fragments).unwrap_or_else(|| panic!("merged class"));
    let raw_code = readable_code(&actual);
    println!(
        "W38H_F195_RAW {}",
        serde_json::json!({
            "schema": 1,
            "kind": "dynamic_fragments",
            "source": source,
            "mode": "merger",
            "code": raw_code,
            "code_utf8_bytes": raw_code.len(),
            "span": [actual.span().start, actual.span().end],
            "stylesheet": null,
        })
    );
    // Then: the head/interior/tail raw bytes and source calls preserve spacing and order.
    let Expression::TemplateLiteral(template) = &actual else {
        panic!("multi-fragment template required")
    };
    let raw: Vec<_> = template
        .quasis
        .iter()
        .map(|quasi| quasi.value.raw.as_str())
        .collect();
    assert_eq!(raw, ["alpha beta ", " ", ""]);
    assert!(
        template
            .quasis
            .iter()
            .all(|quasi| quasi.value.cooked.is_none())
    );
    assert_eq!(
        template
            .expressions
            .iter()
            .map(readable_code)
            .collect::<Vec<_>>(),
        ["first()", "second()"]
    );
    let setup = "function first(){trace.push('first');return 'one';}function second(){trace.push('second');return 'two';}";
    assert_eq!(
        json_evaluated(&actual, setup),
        serde_json::json!(["alpha beta one two", ["first", "second"]])
    );
}
