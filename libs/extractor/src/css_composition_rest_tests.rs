use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::extract_style::style_property::StyleProperty;
use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, ImportAlias, extract};
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

fn compile(body: &str) -> ExtractOutput {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    extract(
        "composition.tsx",
        &format!("import {{ css }} from '@devup-ui/react';\n{body}"),
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{body}: {error}"))
}

fn result_styles(output: &ExtractOutput, condition: bool) -> Vec<ExtractStaticStyle> {
    fn classes(expression: &Expression<'_>, condition: bool) -> String {
        match expression {
            Expression::StringLiteral(literal) => literal.value.to_string(),
            Expression::ConditionalExpression(choice) => classes(
                if condition {
                    &choice.consequent
                } else {
                    &choice.alternate
                },
                condition,
            ),
            Expression::TemplateLiteral(template) => {
                let mut result = String::new();
                for (index, quasi) in template.quasis.iter().enumerate() {
                    result.push_str(quasi.value.raw.as_str());
                    if let Some(expression) = template.expressions.get(index) {
                        result.push_str(&classes(expression, condition));
                    }
                }
                result
            }
            _ => panic!("unexpected compiled classes: {expression:?}"),
        }
    }
    let source = output
        .code
        .split("const result = ")
        .nth(1)
        .unwrap_or_else(|| panic!("missing result: {}", output.code));
    let source = source.trim().trim_end_matches(';');
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let classes = classes(&expression, condition);
    let mut styles: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => match style.extract(None) {
                StyleProperty::ClassName(name)
                    if classes.split_whitespace().any(|class| class == name) =>
                {
                    Some(style.clone())
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    styles.sort();
    styles
}

#[rstest]
#[case("css(css`color: red; padding: 4px;`, { color: 'blue', m: 2 })", "blue")]
#[case("css({ color: 'blue', m: 2 }, css`color: red; padding: 4px;`)", "red")]
#[case(
    "css`color: blue; margin: 8px; ${css`color: red; padding: 4px;`}`",
    "red"
)]
#[case(
    "css`${css`color: red; padding: 4px;`} color: blue; margin: 8px;`",
    "blue"
)]
#[case(
    "css({ color: 'blue', m: 2 }, css(...[{ color: 'red', p: 1 }]))",
    "red"
)]
#[case(
    "css(css(...[{ color: 'red', p: 1 }]), { color: 'blue', m: 2 })",
    "blue"
)]
#[case(
    "css({ color: 'blue', m: 2 }, css(...[, ...[{ color: 'red', p: 1 }], null]))",
    "red"
)]
#[case(
    "css`color: blue; margin: 8px; ${css(...[{ color: 'red', p: 1 }])}`",
    "red"
)]
#[test]
#[serial]
fn later_styles_win_when_inline_parts_compose(#[case] expression: &str, #[case] color: &str) {
    let source = format!("export const result = {expression};");
    let output = compile(&source);
    let styles = result_styles(&output, true);
    let declarations: Vec<_> = styles
        .iter()
        .map(|style| (style.property.as_str(), style.value.as_str()))
        .collect();
    assert_eq!(
        declarations,
        vec![("color", color), ("margin", "8px"), ("padding", "4px")]
    );
    assert!(!output.code.contains("css("), "{}", output.code);
    assert!(!output.code.contains("css`"), "{}", output.code);
}

#[rstest]
#[case(
    "css({ color: 'green', m: 2 }, flag ? css`color: red; padding: 4px;` : css`color: blue; padding: 4px;`)",
    "red",
    "blue"
)]
#[case(
    "css(flag && css`color: red; padding: 4px;`, { color: 'blue', m: 2 })",
    "blue",
    "blue"
)]
#[case(
    "css`color: green; margin: 8px; ${flag ? css`color: red;` : css`color: blue;`}`",
    "red",
    "blue"
)]
#[case(
    "css(css`color: red; ${flag && css`color: green;`}`, { color: 'blue', m: 2 })",
    "blue",
    "blue"
)]
#[case(
    "css({ color: 'blue', m: 2 }, css`color: red; ${flag && css`color: green;`}`)",
    "green",
    "red"
)]
#[test]
#[serial]
fn choices_compose_when_inline_templates_are_conditional(
    #[case] expression: &str,
    #[case] yes: &str,
    #[case] no: &str,
) {
    let output = compile(&format!("export const result = {expression};"));
    for (condition, color) in [(true, yes), (false, no)] {
        let styles = result_styles(&output, condition);
        let colors: Vec<_> = styles
            .iter()
            .filter(|style| style.property == "color")
            .map(|style| style.value.as_str())
            .collect();
        assert_eq!(colors, vec![color]);
        assert!(
            styles
                .iter()
                .any(|style| style.property == "margin" && style.value == "8px")
        );
    }
}

#[test]
#[serial]
fn spread_arrays_compose_when_constants_are_exact() {
    let output = compile(
        "const first = [{ color: 'red', p: 1 }];\nconst parts = [...first, , { color: 'green' }];\nexport const result = css(css(...parts), { color: 'blue', m: 2 });",
    );
    let styles = result_styles(&output, true);
    assert_eq!(
        styles
            .iter()
            .map(|style| (style.property.as_str(), style.value.as_str()))
            .collect::<Vec<_>>(),
        vec![("color", "blue"), ("margin", "8px"), ("padding", "4px")]
    );
}

#[test]
#[serial]
fn external_literals_stay_classes_when_their_text_matches_generated_classes() {
    let output = compile(
        "const base = css`color: red;`;\nexport const result = css('a', css`color: blue;`);",
    );
    assert!(output.code.contains("\"a "), "{}", output.code);
}

#[rstest]
#[case(
    "css(css`color: red; &:hover { color: orange; }`, { color: 'blue', _focus: { color: 'green' } })"
)]
#[case(
    "css({ color: 'blue', _focus: { color: 'green' } }, css`color: red; &:hover { color: orange; }`)"
)]
#[test]
#[serial]
fn selectors_stay_distinct_when_inline_templates_compose(#[case] expression: &str) {
    let output = compile(&format!("export const result = {expression};"));
    let styles = result_styles(&output, true);
    assert_eq!(styles.len(), 3);
    for (selector, value) in [(":hover", "orange"), (":focus", "green")] {
        assert!(
            styles.iter().any(|style| style.value == value
                && style
                    .selector
                    .as_ref()
                    .is_some_and(|actual| actual.to_string().contains(selector))),
            "{styles:?}"
        );
    }
}

#[test]
#[serial]
fn units_stay_library_specific_when_nested_spreads_compose() {
    let option = ExtractOption {
        single_css: true,
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let output = extract("composition.tsx", "import { css } from '@emotion/react';\nexport const result = css(css(...[{ padding: 2, p: 3, m: 3, opacity: 0.5 }]), { padding: 5 });", option).unwrap_or_else(|error| panic!("{error}"));
    let styles = result_styles(&output, true);
    assert_eq!(
        styles
            .iter()
            .map(|style| (style.property.as_str(), style.value.as_str()))
            .collect::<Vec<_>>(),
        vec![("margin", "12px"), ("opacity", ".5"), ("padding", "5px")]
    );
}

#[test]
#[serial]
fn breakpoints_stay_distinct_when_nested_spreads_override_base_styles() {
    let output = compile(
        "export const result = css(css(...[{ color: ['red', null, 'green'] }]), css`color: blue;`);",
    );
    let styles = result_styles(&output, true);
    assert_eq!(
        styles
            .iter()
            .map(|style| (style.value.as_str(), style.level))
            .collect::<Vec<_>>(),
        vec![("blue", 0), ("green", 2)]
    );
}

#[test]
#[serial]
fn layers_stay_distinct_when_nested_spreads_override_matching_rules() {
    use oxc_ast_visit::VisitMut;
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, "import { css } from '@devup-ui/react';\nimport { first, second, theme } from './layers';\nexport const result = css(css(...[first, theme]), second);", SourceType::tsx()).parse().program;
    let mut visitor = crate::visit::DevupVisitor::new(
        &allocator,
        "composition.tsx",
        "@devup-ui/react",
        vec![],
        None,
    );
    visitor.import_css(
        [
            ("first", "red", "base"),
            ("second", "blue", "base"),
            ("theme", "green", "theme"),
        ]
        .into_iter()
        .map(|(name, value, layer)| {
            (
                name.to_string(),
                vec![ExtractStyleValue::Static(
                    ExtractStaticStyle::new_with_layer(
                        "color",
                        value,
                        0,
                        None,
                        Some(layer.to_string()),
                    ),
                )],
            )
        })
        .collect(),
    );
    visitor.visit_program(&mut program);
    assert_eq!(visitor.errors, vec![]);
    let output = ExtractOutput {
        styles: visitor.styles,
        code: oxc_codegen::Codegen::new().build(&program).code,
        map: None,
        css_file: None,
        dependencies: vec![],
    };
    let styles = result_styles(&output, true);
    assert_eq!(
        styles
            .iter()
            .map(|style| (style.value.as_str(), style.layer.as_deref()))
            .collect::<Vec<_>>(),
        vec![("blue", Some("base")), ("green", Some("theme"))]
    );
}

#[rstest]
#[case("css(css`color: ${runtime};`, { m: 2 })")]
#[case("css`color: blue; ${css`&${runtime} { color: red; }`}`")]
#[case("css({ color: 'red' }, css(...[...rest]))")]
#[test]
#[serial]
fn runtime_only_parts_fail_when_the_build_cannot_read_styles(#[case] expression: &str) {
    let source =
        format!("import {{ css }} from '@devup-ui/react';\nexport const result = {expression};");
    let error = extract("composition.tsx", &source, ExtractOption::default())
        .err()
        .unwrap_or_else(|| panic!("expected build error: {source}"))
        .to_string();
    assert!(error.starts_with("composition.tsx:2:"), "{error}");
}

#[rstest]
#[case("css(...[first, second])", "blue")]
#[case("css(...[second, first])", "red")]
#[case("css(...[, ...[first, , second], ,])", "blue")]
#[case("css(...[, ...[second, , first], ,])", "red")]
#[case("css(first, ...parts)", "blue")]
#[case("css(second, ...reverse)", "red")]
#[case("css(...[css(...[first]), second])", "blue")]
#[case("css(...[second, css(...[first])])", "red")]
#[case("css(...[first], second)", "blue")]
#[case("css(second, ...[first])", "red")]
#[test]
#[serial]
fn later_styles_win_when_known_classes_are_argument_spreads(
    #[case] expression: &str,
    #[case] color: &str,
) {
    // Given
    let source = format!(
        "const first = css({{ color: 'red', p: 1 }});\nconst second = css({{ color: 'blue', m: 2 }});\nconst parts = [...[{{ color: 'red', p: 1 }}], , {{ color: 'blue', m: 2 }}];\nconst reverse = [...[{{ color: 'blue', m: 2 }}], , {{ color: 'red', p: 1 }}];\nexport const result = {expression};"
    );
    // When
    let output = compile(&source);
    // Then
    assert_eq!(
        result_styles(&output, true)
            .iter()
            .map(|style| (style.property.as_str(), style.value.as_str()))
            .collect::<Vec<_>>(),
        vec![("color", color), ("margin", "8px"), ("padding", "4px")]
    );
}

#[test]
#[serial]
fn frame_selectors_survive_when_known_classes_are_argument_spreads() {
    // Given
    let source = "import { styled } from '@devup-ui/react';\nconst Frame = styled('div', { p: 1 });\nconst first = css`${Frame} { color: red; padding: 4px; }`;\nconst second = css`${Frame} { color: blue; }`;\nexport const result = css(...[first, second]);";
    // When
    let output = compile(source);
    let styles = result_styles(&output, true);
    // Then
    assert_eq!(
        styles
            .iter()
            .map(|style| (style.property.as_str(), style.value.as_str()))
            .collect::<Vec<_>>(),
        vec![("color", "blue"), ("padding", "4px")]
    );
    assert!(styles.iter().all(|style| style.selector.is_some()));
    assert_eq!(styles[0].selector, styles[1].selector);
}

#[rstest]
#[case("css({ color: 'red' }, css(...rest))", "composition.tsx:2:45:")]
#[case("css`color: red; ${css(...rest)}`", "composition.tsx:2:41:")]
#[case("css(...rest)", "composition.tsx:2:23:")]
#[case("css(...[...rest])", "composition.tsx:2:23:")]
#[test]
#[serial]
fn unknown_spreads_fail_at_the_nested_call_when_arrays_are_runtime_only(
    #[case] expression: &str,
    #[case] location: &str,
) {
    let source =
        format!("import {{ css }} from '@devup-ui/react';\nexport const result = {expression};");
    let error = extract("composition.tsx", &source, ExtractOption::default())
        .err()
        .unwrap_or_else(|| panic!("expected build error: {source}"))
        .to_string();
    assert!(error.starts_with(location), "{error}");
    assert!(error.contains("...rest"), "{error}");
}
