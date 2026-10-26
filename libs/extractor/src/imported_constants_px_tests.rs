use super::*;
use rstest::rstest;

#[rstest]
#[case("good = 2")]
#[case("good")]
#[serial_test::serial]
fn ordinary_enum_member_after_string_key_is_static(
    #[case] member: &str,
    #[values("plain", r"\uD800", r"\uDC00")] key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: a numeric member follows a string-named member in a literal enum.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\nenum E {{ '{key}' = 1, {member} }}\nexport const result = css({{ p: E.good }});"
    );

    // When: the public style surface consumes only the later ordinary member.
    let output = crate::extract("enum-key.tsx", &source, ExtractOption::default())?;

    // Then: the prior key cannot turn the exact static value into a fallback.
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![("padding", "8px")]);
    Ok(())
}

#[rstest]
#[case("``", vec![], Some(""))]
#[case("`a${token}z`", vec![Constant::String("b".to_string())], Some("abz"))]
#[case("`한${token}😀`", vec![Constant::String("글".to_string())], Some("한글😀"))]
#[case("`${token}`", vec![Constant::Number(1e21)], Some("1e+21"))]
#[case("`${token}`", vec![Constant::Bool(true)], Some("true"))]
#[case("`${a}:${b}`", vec![Constant::Null, Constant::Undefined], Some("null:undefined"))]
#[case(r"`\uD83D${token}\uDE00`", vec![Constant::String(String::new())], Some("😀"))]
#[case(r"`left\uD83D${token}\uDE00right`", vec![Constant::String(String::new())], Some("left😀right"))]
#[case(r"`\uD800${token}`", vec![Constant::String(String::new())], None)]
#[case(r"`${token}\uDC00`", vec![Constant::String(String::new())], None)]
#[case(r"`\uD83D${token}\uDE00`", vec![Constant::Number(1.0)], None)]
#[case("`${token}`", vec![Constant::Array(Rc::new(vec![]))], None)]
#[case(r"`\uD800${token}`", vec![Constant::Array(Rc::new(vec![]))], None)]
fn folded_template_keeps_complete_static_text(
    #[case] source: &str,
    #[case] values: Vec<Constant>,
    #[case] expected: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: valid authored template syntax and the evaluator's constant substitutions.
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::mjs())
        .parse_expression()
        .map_err(|errors| format!("invalid template fixture: {errors:?}"))?;
    let Expression::TemplateLiteral(template) = expression else {
        return Err("fixture must be a template literal".into());
    };

    // When: the constant folder assembles the complete JavaScript string.
    let result = fold_template(&template, &values);

    // Then: exact text survives, but incomplete surrogates and unsupported values do not fold.
    match (result, expected) {
        (Some(Constant::String(actual)), Some(expected)) => assert_eq!(actual, expected),
        (None, None) => {}
        (actual, expected) => {
            panic!("unexpected template result: {actual:?}, expected {expected:?}")
        }
    }
    Ok(())
}

#[test]
fn own_css_record_keeps_fallback_order_when_authored_px_mark_is_inlined() {
    // Given: Emotion-style fallback arrays, not responsive breakpoints.
    let source = "import { css } from '@devup-ui/react';
const rules = { padding: [2, 4], opacity: [0, 0.5] };
export const card = css`${rules}`;";
    let allocator = Allocator::default();
    let mut program = crate::parsing::parse(&allocator, "/src/fallback.ts", source)
        .unwrap_or_else(|error| panic!("invalid fallback fixture: {error}"));
    let mut marks = crate::import_alias_visit::PxMarks::default();
    let start = u32::try_from(
        source
            .rfind("rules}")
            .unwrap_or_else(|| panic!("missing interpolation")),
    )
    .unwrap_or_else(|error| panic!("fixture offset exceeds u32: {error}"));
    marks.insert((start, start + 5));
    // When
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "/src/fallback.ts",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &marks,
    );
    // Then: preserve ordered fallbacks, adding px only to length values.
    assert_eq!(inlined.css_failures.len(), 0);
    let Some(Statement::ExportDeclaration(export)) = program.body.last() else {
        panic!("expected card")
    };
    let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration else {
        panic!("expected declaration")
    };
    let Some(Expression::TaggedTemplateExpression(tagged)) =
        declaration.declarations[0].init.as_ref()
    else {
        panic!("expected tag")
    };
    let Expression::ObjectExpression(object) = &tagged.quasi.expressions[0] else {
        panic!("expected materialized rules")
    };
    let values: Vec<_> = object
        .properties
        .iter()
        .map(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                panic!("expected property")
            };
            let Expression::StringLiteral(value) = &property.value else {
                panic!("expected ordered fallback")
            };
            (
                property
                    .key
                    .static_name()
                    .unwrap_or_else(|| panic!("expected static key"))
                    .into_cow_str()
                    .unwrap_or_else(|| panic!("expected UTF-8 key"))
                    .into_owned(),
                value
                    .value
                    .as_str()
                    .unwrap_or_else(|| panic!("expected UTF-8 fallback"))
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(
        values,
        [
            ("padding".to_string(), "2px;padding:4px".to_string()),
            ("opacity".to_string(), "0;opacity:0.5".to_string())
        ]
    );
}

#[rstest]
fn own_css_record_honors_authored_px_marks_when_inlined(#[values(false, true)] marked: bool) {
    // Given: only the authored interpolation is marked, not its declaration.
    let source = "import { css } from '@devup-ui/react';
const rules = { padding: 2, p: 3, opacity: 0.5, margin: 0, vars: { '--gap': 7 } };
export const card = css`${rules}`;";
    let allocator = Allocator::default();
    let mut program = crate::parsing::parse(&allocator, "/src/px.ts", source)
        .unwrap_or_else(|error| panic!("invalid px fixture: {error}"));
    let mut marks = crate::import_alias_visit::PxMarks::default();
    let start = u32::try_from(
        source
            .rfind("rules}")
            .unwrap_or_else(|| panic!("missing authored interpolation")),
    )
    .unwrap_or_else(|error| panic!("fixture offset exceeds u32: {error}"));
    if marked {
        marks.insert((start, start + 5));
    }

    // When: run the lower-level contract directly, without an alias transform.
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "/src/px.ts",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &marks,
    );

    // Then: marked lengths become px, but unitless, zero and vars stay numeric.
    assert_eq!(inlined.css_failures.len(), 0);
    let Some(Statement::ExportDeclaration(export)) = program.body.last() else {
        panic!("expected exported card");
    };
    let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration else {
        panic!("expected card declaration");
    };
    let Some(Expression::TaggedTemplateExpression(tagged)) =
        declaration.declarations[0].init.as_ref()
    else {
        panic!("expected own css tag");
    };
    let Expression::ObjectExpression(object) = &tagged.quasi.expressions[0] else {
        panic!("expected materialized record");
    };
    for (property, (key, number)) in object.properties.iter().zip([
        ("padding", 2.0),
        ("p", 3.0),
        ("opacity", 0.5),
        ("margin", 0.0),
        ("vars", 7.0),
    ]) {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            panic!("expected record property");
        };
        assert_eq!(
            property
                .key
                .static_name()
                .and_then(oxc_ast::StaticPropertyName::into_cow_str)
                .as_deref(),
            Some(key)
        );
        let value = if key == "vars" {
            let Expression::ObjectExpression(vars) = &property.value else {
                panic!("expected vars record");
            };
            let ObjectPropertyKind::ObjectProperty(variable) = &vars.properties[0] else {
                panic!("expected custom property");
            };
            &variable.value
        } else {
            &property.value
        };
        if marked && key == "padding" {
            let Expression::StringLiteral(value) = value else {
                panic!("expected marked length in px, got {value:?}");
            };
            assert_eq!(value.value, "2px");
        } else {
            assert_eq!(crate::utils::js_number_literal(value), Some(number));
        }
    }
    assert_eq!(object.properties.len(), 5);
}
