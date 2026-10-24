use super::exact_tests::{extracted, static_values};
use super::{Constant, constant_literal};

#[rstest::rstest]
#[case(("Math.sign(-0)", "green"))]
#[case(("Math.fround(-0)", "green"))]
#[case(("Math.min(0, -0)", "green"))]
#[case(("Math.sign(0)", "red"))]
#[case(("Math.fround(0)", "red"))]
#[case(("Math.max(-0, 0)", "red"))]
#[serial_test::serial]
fn computed_color_when_imported_zero_is_consumed(
    #[case] scenario: (&str, &str),
    #[values("Object.is(n, -0)", "1 / n === -Infinity")] predicate: &str,
    #[values(false, true)] inline: bool,
) {
    // Given
    let (expression, expected) = scenario;
    let module = format!("export const VALUE = {expression};");
    let consumer = if inline {
        "export const s = css({ color: colorOf(VALUE) });"
    } else {
        "const color = colorOf(VALUE); export const s = css({ color });"
    };
    let code = format!(
        "import {{ css }} from '@devup-ui/react'; import {{ VALUE }} from './values';
         const colorOf = n => {predicate} ? 'green' : 'red'; {consumer}"
    );

    // When
    let output = extracted(&code, &module).unwrap_or_else(|error| panic!("{code}: {error}"));

    // Then
    assert_eq!(
        static_values(&output),
        vec![expected.to_string()],
        "{expression}, {predicate}, inline={inline}"
    );
}

#[rstest::rstest]
#[case("Math.sign(-0)")]
#[case("Math.fround(-0)")]
#[case("Math.min(0, -0)")]
#[case("Math.sign(0)")]
#[serial_test::serial]
fn canonical_css_zero_when_imported_zero_is_a_style_value(#[case] expression: &str) {
    // Given
    let module = format!("export const VALUE = {expression};");
    let code = "import { css } from '@devup-ui/react'; import { VALUE } from './values';
                export const s = css({ zIndex: VALUE });";

    // When
    let output = extracted(code, &module).unwrap_or_else(|error| panic!("{expression}: {error}"));

    // Then
    assert_eq!(static_values(&output), vec!["0".to_string()]);
}

#[rstest::rstest]
#[case(-0.0, "green")]
#[case(0.0, "red")]
#[serial_test::serial]
fn computed_color_when_scalar_ast_is_serialized(#[case] zero: f64, #[case] expected: &str) {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let builder = oxc_ast::builder::AstBuilder::new(&allocator);
    let literal = constant_literal(&builder, &Constant::Number(zero), false)
        .unwrap_or_else(|| panic!("finite scalar must have a literal"));
    let mut codegen = oxc_codegen::Codegen::new();
    codegen.print_expression(&literal);
    let code = format!(
        "import {{ css }} from '@devup-ui/react';
         const colorOf = n => Object.is(n, -0) ? 'green' : 'red';
         export const s = css({{ color: colorOf({}) }});",
        codegen.into_source_text()
    );

    // When
    let output = extracted(&code, "").unwrap_or_else(|error| panic!("{code}: {error}"));

    // Then
    assert_eq!(static_values(&output), vec![expected.to_string()]);
}
