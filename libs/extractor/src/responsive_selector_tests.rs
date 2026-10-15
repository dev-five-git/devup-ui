use crate::{ExtractOption, extract, extract_style::extract_style_value::ExtractStyleValue};
use css::{class_map::reset_class_map, file_map::reset_file_map, style_selector::StyleSelector};
use rstest::rstest;
use serial_test::serial;
use std::collections::BTreeSet;

#[rstest]
#[case("[[1, 2], 3]", "[1, 2]")]
#[case("[0, [1, 2]]", "[1, 2]")]
#[case("[[[1, 2]], 3]", "[[1, 2]]")]
#[case("[[], 3]", "[]")]
#[case("[([1, 2]), 3]", "[1, 2]")]
#[case("[([1, 2] as const), 3]", "[1, 2]")]
#[case("[([1, 2] satisfies number[]), 3]", "[1, 2]")]
#[case("[([1, 2]!), 3]", "[1, 2]")]
#[case("[on ? [1, 2] : 3, 4]", "[1, 2]")]
#[case("[on ? 3 : ([1, 2] as const), 4]", "[1, 2]")]
#[case("[on && [1, 2], 4]", "[1, 2]")]
#[case("[true ? [1, 2] : 3, 4]", "[1, 2]")]
#[serial]
fn nested_responsive_arrays_are_errors(
    #[case] value: &str,
    #[case] nested: &str,
    #[values("jsx", "css", "styled")] api: &str,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let statement = match api {
        "jsx" => format!("const e = <Box p={{{value}}} />;"),
        "css" => format!("const c = css({{ p: {value} }});"),
        "styled" => format!("const S = styled.div({{ p: {value} }});"),
        _ => unreachable!(),
    };
    let source = format!("import {{ Box, css, styled }} from '@devup-ui/react';\n{statement}");
    let column = crate::dead_properties_test_utils::position(&statement, nested) + 1;
    let owner = match api {
        "jsx" => "`<Box>`",
        "css" => "`css()`",
        "styled" => "`styled()`",
        _ => unreachable!(),
    };
    // When
    let error = crate::dead_properties_test_utils::failure(extract(
        "edges.tsx",
        &source,
        ExtractOption::default(),
    ));
    // Then
    assert_eq!(
        error,
        format!(
            "edges.tsx:2:{column}: {owner} cannot use `{nested}` at build time: responsive arrays must be flat; each entry supplies one breakpoint value or selector style object, not another array"
        )
    );
}

#[rstest]
#[case("const e = <Box p={[1, null, 2, , 3]} />;")]
#[case("const c = css({ p: [1, null, 2, , 3] });")]
#[case("const S = styled.div({ p: [1, null, 2, , 3] });")]
#[case("const e = <Box p={[false ? [9, 9] : 1, null, 2, , 3]} />;")]
#[serial]
fn flat_responsive_arrays_preserve_property_and_breakpoint(
    #[case] statement: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ Box, css, styled }} from '@devup-ui/react';\n{statement}");
    // When
    let output = extract("edges.tsx", &source, ExtractOption::default())?;
    // Then
    let styles: BTreeSet<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => {
                assert_eq!(style.selector, None);
                (style.property.as_str(), style.level, style.value.as_str())
            }
            other => panic!("expected static responsive style, got {other:?}"),
        })
        .collect();
    assert_eq!(
        styles,
        BTreeSet::from([
            ("padding", 0, "4px"),
            ("padding", 2, "8px"),
            ("padding", 4, "12px")
        ])
    );
    Ok(())
}

#[rstest]
#[case("idx", vec![(0, "4px"), (1, "8px"), (2, "12px"), (0, "16px"), (1, "20px"), (2, "24px")])]
#[case("1", vec![(0, "16px"), (1, "20px"), (2, "24px")])]
#[serial]
fn indexed_arrays_select_one_responsive_array(
    #[case] index: &str,
    #[case] expected: Vec<(u8, &str)>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ Box }} from '@devup-ui/react';\nconst e = <Box p={{[[1,2,3],[4,5,6]][{index}]}} />;"
    );
    // When
    let output = extract("edges.tsx", &source, ExtractOption::default())?;
    // Then
    let styles: BTreeSet<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => {
                assert_eq!(style.property, "padding");
                assert_eq!(style.selector, None);
                (style.level, style.value.as_str())
            }
            other => panic!("expected static indexed style, got {other:?}"),
        })
        .collect();
    assert_eq!(styles, expected.into_iter().collect());
    Ok(())
}

#[rstest]
#[case("<Box _hover={{ p: [1, null, 2] }} />")]
#[case("<Box _hover={[{ p: 1 }, null, { p: 2 }]} />")]
#[case("<Box selectors={{ ':hover': { p: [1, null, 2] } }} />")]
#[case("<Box selectors={{ '&:hover': { p: [1, null, 2] } }} />")]
#[serial]
fn hover_responsive_styles_are_scoped(
    #[case] element: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ Box }} from '@devup-ui/react';\nconst e = {element};");
    // When
    let output = extract("edges.tsx", &source, ExtractOption::default())?;
    // Then
    let styles: BTreeSet<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => {
                assert_eq!(
                    style.selector,
                    Some(StyleSelector::Selector("&:hover".to_string()))
                );
                (style.property.as_str(), style.level, style.value.as_str())
            }
            other => panic!("expected scoped static style, got {other:?}"),
        })
        .collect();
    assert_eq!(
        styles,
        BTreeSet::from([("padding", 0, "4px"), ("padding", 2, "8px")])
    );
    Ok(())
}

#[test]
#[serial]
fn body_selector_is_a_located_build_error() {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import { Box } from '@devup-ui/react';\nconst e = <Box selectors={{ body: { color: 'pink' } }} />;";
    // When
    let error = crate::dead_properties_test_utils::failure(extract(
        "edges.tsx",
        source,
        ExtractOption::default(),
    ));
    // Then
    assert_eq!(
        error,
        format!(
            "edges.tsx:2:29: `<Box>` cannot use `body` at build time: {}",
            crate::utils::SELECTOR_NAME
        )
    );
}

#[test]
#[serial]
fn raw_parent_selector_preserves_scope() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import { Box } from '@devup-ui/react';\nconst e = <Box selectors={{ '& > p': { color: 'pink' } }} />;";
    // When
    let output = extract("edges.tsx", source, ExtractOption::default())?;
    // Then
    assert_eq!(output.styles.len(), 1);
    let style = output
        .styles
        .iter()
        .next()
        .ok_or("expected one static style")?;
    match style {
        ExtractStyleValue::Static(style) => {
            assert_eq!(style.property, "color");
            assert_eq!(style.value, "pink");
            assert_eq!(style.level, 0);
            assert_eq!(
                style.selector,
                Some(StyleSelector::Selector("& > p".to_string()))
            );
        }
        other => panic!("expected scoped child style, got {other:?}"),
    }
    Ok(())
}

#[rstest]
#[case(
    "<Box color={[[\"red\", \"blue\"], \"green\"]} />",
    "[\"red\", \"blue\"]"
)]
#[case("<Box _hover={[[{ p: 1 }], { p: 2 }]} />", "[{ p: 1 }]")]
#[case("<Box _hover={{ p: [[1, 2], 3] }} />", "[1, 2]")]
#[case("<Box selectors={{ '&:hover': { p: [[1, 2], 3] } }} />", "[1, 2]")]
#[serial]
fn nested_responsive_selector_values_are_errors(#[case] element: &str, #[case] nested: &str) {
    // Given
    reset_class_map();
    reset_file_map();
    let statement = format!("const e = {element};");
    let source = format!("import {{ Box }} from '@devup-ui/react';\n{statement}");
    let column = crate::dead_properties_test_utils::position(&statement, nested) + 1;
    // When
    let error = crate::dead_properties_test_utils::failure(extract(
        "edges.tsx",
        &source,
        ExtractOption::default(),
    ));
    // Then
    assert_eq!(
        error,
        format!(
            "edges.tsx:2:{column}: `<Box>` cannot use `{nested}` at build time: {}",
            crate::utils::RESPONSIVE_ARRAY
        )
    );
}

#[test]
fn nested_array_diagnostic_retains_its_exact_span() {
    use crate::extractor::extract_style_from_expression::{
        LiteralHandling, extract_style_from_expression,
    };
    use oxc_allocator::Allocator;
    use oxc_ast::{ast::Statement, builder::AstBuilder};
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    // Given
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = "[0, ([1, 2] as const)];";
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("expected array expression statement");
    };
    // When
    let result = extract_style_from_expression(
        &builder,
        Some("padding"),
        &mut statement.expression,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    );
    // Then
    let mut diagnostics = vec![];
    crate::utils::unreadable_styles(&result.styles, false, &mut diagnostics);
    assert_eq!(
        diagnostics,
        vec![(
            5,
            "[1, 2]".to_string(),
            Some(crate::utils::RESPONSIVE_ARRAY)
        )]
    );
}
