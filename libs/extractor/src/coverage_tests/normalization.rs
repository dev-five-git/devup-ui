use super::{code, dynamic, expression};
use crate::{ExtractStyleProp, assignment_test_support::evaluate};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use serial_test::serial;

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
