use super::*;
use crate::extract_style::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};
use rstest::rstest;

pub(super) fn inventory(actual: &ExtractOutput, expected: &[(&str, &str, u8, Option<u8>)]) {
    let mut authored: Vec<_> = expected
        .iter()
        .map(|(property, value, level, order)| {
            ExtractStyleValue::Static(ExtractStaticStyle {
                property: (*property).into(),
                value: (*value).into(),
                level: *level,
                selector: None,
                style_order: *order,
                layer: None,
                theme_token_resolution: ThemeTokenResolution::CssVariable,
            })
        })
        .collect();
    authored.sort_unstable();
    let mut found: Vec<_> = actual.styles.iter().cloned().collect();
    found.sort_unstable();
    assert_eq!(found, authored);
}

#[rstest]
#[case::arrow(
    "p=>p.active?'external':false",
    "color-0-red--255-a external",
    "color-0-red--255-a "
)]
#[case::function(
    "function(p){return p.active?'external':false}",
    "color-0-red--255-a external",
    "color-0-red--255-a "
)]
#[case::class(
    "external",
    "color-0-red--255-a external",
    "color-0-red--255-a external"
)]
#[serial]
fn w38r_u2_nested_statement_when_tag_survives_preserves_class_and_callback_fallback(
    #[case] mixin: &str,
    #[case] yes: &str,
    #[case] no: &str,
) {
    // Given: nested statement cannot be segmented; legacy extraction keeps its class.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=r=>r;let external='external';const Card=styled.div`color:red;&:hover{{${{{mixin}}};}}`;const a=Card({{active:true}},null);const b=Card({{active:false}},null);"
    );
    // When: extract and render the surviving tagged-template path.
    let actual = output(&source);
    let evaluated = whole::evaluate_code(&actual.code, "[a.props.className,b.props.className]");
    // Then: callbacks are invoked with props and false contributes no class.
    assert_eq!(evaluated.element, serde_json::json!([yes, no]));
    inventory(&actual, &[("color", "red", 0, None)]);
}

#[rstest]
#[case::async_arrow("async p=>({color:p.color})")]
#[serial]
fn w38r_u2_async_expression_callback_when_not_build_readable_reports_located_error(
    #[case] callback: &str,
) {
    // Given: asynchronous rule callbacks do not synchronously return rule objects.
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({callback});");
    // When: compile through the same public extraction entry as valid callbacks.
    let actual = error(&source);
    // Then: reject the original callback at the styled call, not generated code.
    assert_eq!(
        actual,
        "a.tsx:2:12: Cannot compose `async (p) => ({ color: p.color })` at build time: each style must be a rule object, a class, or a condition choosing between them"
    );
}

#[cfg(test)]
#[path = "w38r_u3_public.rs"]
mod w38r_u3_public;

#[cfg(test)]
#[path = "w38r_u4_public.rs"]
mod w38r_u4_public;

#[cfg(test)]
#[path = "w38r_u5_public.rs"]
mod w38r_u5_public;

#[cfg(test)]
#[path = "w38r_u5_imports.rs"]
mod w38r_u5_imports;

#[cfg(test)]
#[path = "w38r_u6_order.rs"]
mod w38r_u6_order;

#[cfg(test)]
#[path = "w38r_u6_nested.rs"]
mod w38r_u6_nested;
