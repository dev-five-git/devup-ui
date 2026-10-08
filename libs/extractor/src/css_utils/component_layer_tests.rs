use super::{CssToStyleResult, css_to_style, css_to_style_template};
use crate::{ExtractOption, ExtractStyleValue, ImportAlias, extract};
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[test]
fn layer_metadata_nests_when_blocks_wrap_selectors_and_media() {
    let text =
        "@layer base { @layer reset { @media print { &:hover { color: red; } } } } color: blue;";
    let styles = css_to_style(text, 2, &None);
    assert_eq!(styles.len(), 2);
    let red = styles
        .iter()
        .find(|style| style.value == "red")
        .unwrap_or_else(|| panic!("missing layered declaration: {styles:?}"));
    assert_eq!(red.layer.as_deref(), Some("base.reset"));
    assert_eq!(red.level, 2);
    assert!(red.selector.as_ref().is_some_and(|selector| {
        let text = selector.to_string();
        text.contains("print") && text.contains(":hover")
    }));
    assert!(
        styles
            .iter()
            .any(|style| style.value == "blue" && style.layer.is_none())
    );
}

#[rstest]
#[case("`@layer base { color: ${'red'}; }`", "red")]
#[case("`@layer base { color: ${runtime.color}; }`", "runtime.color")]
#[case("`@layer base { width: ${runtime.width}px; }`", "`${runtime.width}px`")]
#[test]
fn layer_metadata_survives_when_template_values_are_rebuilt(
    #[case] source: &str,
    #[case] value: &str,
) {
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::TemplateLiteral(template) = expression else {
        panic!("expected template")
    };
    let parsed = css_to_style_template(&template, 0, &None);
    assert_eq!(parsed.styles.len(), 1);
    match &parsed.styles[0] {
        CssToStyleResult::Static(style) => {
            assert_eq!(style.layer.as_deref(), Some("base"));
            assert_eq!(style.value, value);
        }
        CssToStyleResult::Dynamic(style) => {
            assert_eq!(style.layer(), Some("base"));
            assert_eq!(style.identifier(), value);
        }
    }
}

#[rstest]
#[case(
    "import { css } from '@devup-ui/react';\nexport const result = css`@layer base { color: red; }`;"
)]
#[case(
    "import { styled } from '@devup-ui/react';\nexport const result = styled.div`@layer base { color: red; }`;"
)]
#[case(
    "import { css } from '@emotion/react';\nexport const result = <div css={`@layer base { color: red; }`} />;"
)]
#[case(
    "import { ClassNames } from '@emotion/react';\nexport const result = <ClassNames>{({css}) => <div className={css`@layer base { color: red; }`} />}</ClassNames>;"
)]
#[test]
#[serial]
fn declarations_are_retained_when_component_apis_read_layer_text(#[case] source: &str) {
    let option = ExtractOption {
        single_css: true,
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let output = extract("layer.tsx", source, option).unwrap_or_else(|error| panic!("{error}"));
    let layers: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((
                style.property.as_str(),
                style.value.as_str(),
                style.layer.as_deref(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(layers, vec![("color", "red", Some("base"))]);
}

#[rstest]
#[case(
    "import { css } from '@devup-ui/react';\nexport const result = css`color: red;\n @layer base, theme;\n color: blue;`;"
)]
#[case(
    "import { css } from '@devup-ui/react';\nexport const result = css(css`color: red;\n @layer base, theme;`, { m: 2 });"
)]
#[case(
    "import { css } from '@devup-ui/react';\nexport const result = css`color: red;\n @layer base, theme; ${css`color: green;`}`;"
)]
#[case(
    "import { css } from '@emotion/react';\nexport const result = <div css={`color: red;\n @layer base, theme;`} />;"
)]
#[case(
    "import { ClassNames } from '@emotion/react';\nexport const result = <ClassNames>{({css}) => <div className={css`color: red;\n @layer base, theme;`} />}</ClassNames>;"
)]
#[case(
    "import { styled } from '@devup-ui/react';\nexport const result = styled.div`color: red;\n @layer base, theme;`;"
)]
#[test]
#[serial]
fn layer_order_is_rejected_at_its_source_when_component_apis_read_statements(#[case] source: &str) {
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let error = extract("layer.tsx", source, option)
        .err()
        .unwrap_or_else(|| panic!("expected component layer-order error: {source}"));
    let error = error.to_string();
    assert!(error.starts_with("layer.tsx:3:2:"), "{error}");
    assert!(error.contains("@layer base, theme;"), "{error}");
    assert!(error.contains("globalCss"), "{error}");
}

#[rstest]
#[case("`color: red; /* @layer a, b; */ content: '@layer a, b;';`")]
#[case("`@layer base { color: red; }`")]
#[test]
fn validation_ignores_order_text_when_it_is_not_a_layer_statement(#[case] source: &str) {
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(super::expression_layer_errors(&expression, "css"), vec![]);
}

#[test]
fn validation_locates_statements_when_comments_precede_them() {
    let source = "`/* before */ @layer base, theme;`";
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let errors = super::expression_layer_errors(&expression, "css");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, 14);
}

#[rstest]
#[case(
    "css(css`@layer base { color: red; } @layer theme { color: green; }`, css`@layer base { color: blue; }`)",
    "blue"
)]
#[case(
    "css(css`@layer base { color: blue; }`, css`@layer base { color: red; } @layer theme { color: green; }`)",
    "red"
)]
#[test]
#[serial]
fn later_declarations_win_only_in_the_same_layer_when_templates_compose(
    #[case] expression: &str,
    #[case] color: &str,
) {
    use crate::extract_style::{ExtractStyleProperty, style_property::StyleProperty};
    let source =
        format!("import {{ css }} from '@devup-ui/react';\nexport const result = {expression};");
    let output = extract(
        "layer.tsx",
        &source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let classes = output
        .code
        .split("const result = ")
        .nth(1)
        .unwrap_or_else(|| panic!("missing result: {}", output.code));
    let classes: String = serde_json::from_str(classes.trim().trim_end_matches(';'))
        .unwrap_or_else(|error| panic!("{error}: {classes}"));
    let mut layers: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => match style.extract(None) {
                StyleProperty::ClassName(name)
                    if classes.split_whitespace().any(|class| class == name) =>
                {
                    Some((style.layer.as_deref(), style.value.as_str()))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    layers.sort();
    assert_eq!(
        layers,
        vec![(Some("base"), color), (Some("theme"), "green")]
    );
}
