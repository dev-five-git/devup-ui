use rstest::rstest;

use super::super::plan::EscapeKind;
use super::{names, selected, text};

#[rstest]
#[case("import {style as make} from '@vanilla-extract/css';const box=make({color:'red'});")]
#[case("import * as ve from '@vanilla-extract/css';const box=ve.style({color:'red'});")]
#[case("import * as ve from '@vanilla-extract/css';const box=ve['style']({color:'red'});")]
#[case(
    concat!("import * as ve from '@vanilla-extract/css';", "const {style:make}=ve;const box=make({color:'red'});")
)]
#[case(
    "import {style} from '@vanilla-extract/css';const make=style;const box=make({color:'red'});"
)]
fn resolves_native_calls_by_lexical_symbols_when_imports_are_renamed_or_aliased(
    #[case] source: &str,
) {
    // Given source above
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(plan.native_calls[0].api, "style");
    assert_eq!(plan.escapes.len(), 0);
    assert_eq!(plan.checks.len(), 0);
}

#[test]
fn ignores_shadowed_named_and_namespace_bindings_even_inside_selected_helpers() {
    // Given
    let source = "import {style,createVar} from '@vanilla-extract/css';import * as ve from '@vanilla-extract/css';function make(style,ve){style({});ve.style({});return createVar()}const token=make(()=>0,{style:()=>0});export function render(style,ve){return style({})+ve['style']({})}";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["make", "token"]);
    assert_eq!(
        plan.native_calls
            .iter()
            .map(|call| call.api)
            .collect::<Vec<_>>(),
        vec!["createVar"]
    );
    assert_eq!(plan.escapes.len(), 0);
}

#[rstest]
#[case(
    "import {style} from '@vanilla-extract/css';export {style};",
    "style",
    EscapeKind::NativeValue
)]
#[case(
    "import {style} from '@vanilla-extract/css';register(style);",
    "style",
    EscapeKind::NativeValue
)]
#[case(
    "import * as ve from '@vanilla-extract/css';const box=ve[key]({});",
    "ve",
    EscapeKind::DynamicNamespace
)]
#[case(
    "import {style} from '@vanilla-extract/css';export function render(){return style({})}",
    "render",
    EscapeKind::RuntimeHelper
)]
fn locates_genuine_runtime_escapes_instead_of_silently_preserving_native_apis(
    #[case] source: &str,
    #[case] cause: &str,
    #[case] kind: EscapeKind,
) {
    // Given source above
    // When
    let plan = selected(source);
    // Then
    assert!(
        plan.escapes
            .iter()
            .any(|escape| escape.kind == kind && text(source, escape.span) == cause),
        "{:#?}",
        plan.escapes
    );
}

#[test]
fn defers_a_dynamic_native_branch_inside_a_selected_helper_to_exact_execution() {
    // Given
    let source = "import * as ve from '@vanilla-extract/css';function make(enabled){if(enabled)return ve.style({color:'blue'});return ve[window.name]({})}const box=make(true);";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.escapes.len(), 0);
    assert_eq!(plan.checks.len(), 1);
    assert_eq!(plan.checks[0].kind, EscapeKind::DynamicNamespace);
    assert_eq!(text(source, plan.checks[0].span), "ve");
}

#[test]
fn records_member_mutations_as_located_observations_not_blanket_helper_errors() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';const tokens={color:'red'};tokens.color=window.name;function read(value){return value.color}const box=style({color:read(tokens)});";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.escapes.len(), 0);
    assert!(plan.mutations.iter().any(|mutation| matches!(&mutation.usage, crate::mutations::Use::Changes { at, .. } if source[usize::try_from(*at).unwrap_or_else(|error| panic!("invalid source offset: {error}"))..].starts_with("tokens.color="))));
}
