use super::{code, dynamic, expression};
use crate::{ExtractStyleProp, assignment_test_support::evaluate};
use oxc_allocator::Allocator;
use oxc_allocator::CloneIn;
use oxc_ast::builder::AstBuilder;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(concat!("`${", "value}` + ';'"), "red", "[[\"red\"],\"red;\"]")]
#[case(concat!("`${", "value}` + ';;;'"), "", "[[\"\"],\";;;\"]")]
#[case(concat!("`${", "value}` + ';'"), "4", "[[\"16px\"],\"4;\"]")]
#[case(concat!("`${", "value}` + ';'"), "0", "[[\"0px\"],\"0;\"]")]
#[case(concat!("`${", "value} !important;;;`"), "red", "[[\"red\"],\"red !important;;;\"]")]
#[case(
    concat!("`${", "value}` + ' !important;;'"),
    "red",
    "[[\"red\"],\"red !important;;\"]"
)]
#[serial]
fn normalization_keeps_raw_tuple_when_fixed_tail_is_removed(
    #[case] authored: &str,
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given: the original AST is retained before real extraction normalizes the identifier.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, authored);
    let mut extracted = source.clone_in(&allocator);
    let mut styles = crate::extractor::extract_style_from_expression::extract_style_from_expression(
        &ast, Some("p"), &mut extracted, 0, &None,
        crate::extractor::extract_style_from_expression::LiteralHandling::ExpandResponsiveThemeToken,
    ).styles;
    let lowering = crate::assignment_lowering::Lowering {
        ast: &ast,
        order: None,
        filename: None,
        alternate_order: None,
    };
    // When: the actual tuple executes with a string value.
    let generated = lowering.lower(&source, &mut styles);
    let actual = evaluate(&format!(
        "const value={value:?};const result={};JSON.stringify([Object.values(result[1]),result[2]]);",
        code(&generated)
    ));
    // Then: CSS sees normalized conversion and slot two sees the authored suffix.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn fixed_template_tail_keeps_raw_suffix_when_string_addition_is_nested() {
    // Given: normalized metadata has removed a fixed template tail from a string expression.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "('' + value) + `;;`");
    let normalized = expression(&allocator, "('' + value) + ``");
    let mut styles = [ExtractStyleProp::Static(dynamic(
        "color",
        &code(&normalized),
    ))];
    let lowering = crate::assignment_lowering::Lowering {
        ast: &ast,
        order: None,
        filename: None,
        alternate_order: None,
    };
    // When: normalized CSS and raw slots execute together.
    let generated = lowering.lower(&source, &mut styles);
    let actual = evaluate(&format!(
        "const value='red';const result={};JSON.stringify([Object.values(result[1]),result[2]]);",
        code(&generated)
    ));
    // Then: only CSS loses the fixed suffix; the authored raw string keeps it.
    assert_eq!(actual, "[[\"red\"],\"red;;\"]");
}

#[test]
#[serial]
fn important_leaf_keeps_clean_inline_value_and_original_raw_suffix() {
    // Given
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "normalized");
    let mut styles = [ExtractStyleProp::Static(dynamic(
        "color",
        "normalized !important",
    ))];
    let lowering = crate::assignment_lowering::Lowering {
        ast: &ast,
        order: None,
        filename: None,
        alternate_order: None,
    };
    // When
    let generated = lowering.lower(&source, &mut styles);
    let actual = evaluate(&format!(
        "const normalized='red';const result={};JSON.stringify([Object.values(result[1]),result[2]]);",
        code(&generated)
    ));
    // Then
    assert_eq!(actual, "[[\"red\"],\"red !important\"]");
}

#[test]
#[serial]
fn risky_partial_class_map_uses_import_dependent_atom_identity() {
    use crate::extract_style::ExtractStyleProperty;

    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = "import {css}from '@devup-ui/react';export const sample=css({color:'red'});";
    let names = rustc_hash::FxHashSet::from_iter(["sample".to_string()]);
    let option = crate::ExtractOption::default();
    // When
    let actual = crate::extract_class_map_from_code(
        "partial.tsx",
        source,
        &option,
        &names,
        css::Naming::Risky,
    )
    .unwrap_or_else(|error| panic!("class map: {error}"));
    // Then: compare to independently constructed risky content, not the output itself.
    let style = crate::extract_style::extract_static_style::ExtractStaticStyle::new(
        "color", "red", 0, None,
    )
    .with_naming(css::Naming::Risky);
    let crate::extract_style::style_property::StyleProperty::ClassName(expected) =
        style.extract(Some("partial.tsx"))
    else {
        panic!("static class")
    };
    assert_eq!(actual.get("sample"), Some(&expected));
}
