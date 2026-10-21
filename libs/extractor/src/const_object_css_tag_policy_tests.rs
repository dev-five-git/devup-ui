use super::*;

#[rstest]
#[case::identifier(("  color:\n    tone,", 4, 5, "tone"))]
#[case::computed_member(("  color:\n    palette[key],", 4, 5, "palette[key]"))]
#[case::call(("  color:\n    chooseTone(),", 4, 5, "chooseTone()"))]
#[case::sequence(("  color:\n    (tick(), tone),", 4, 6, "(tick(),tone)"))]
#[case::repeated_dead_branch(("  color: false ? tone : 'red',\n  background:\n    tone,", 5, 5, "tone"))]
#[case::responsive(("  color: ['red',\n    tone],", 4, 5, "tone"))]
#[case::selector(("  _hover: { color:\n    tone },", 4, 5, "tone"))]
#[case::at_rule(("  '@media': { '(min-width: 1px)': { color:\n    tone } },", 4, 5, "tone"))]
#[case::layer(("  '@layer': { base: { color:\n    tone } },", 4, 5, "tone"))]
#[case::member_selected_object(("  color: ({ active:\n    tone, dead: 'red' })['active'],", 4, 5, "tone"))]
#[case::member_selected_array(("  color: [\n    tone, 'red'][0],", 4, 5, "tone"))]
#[case::member_runtime_spread(("  color:\n    ['red', ...tones][1],", 4, 5, "[`red`,...tones][1]"))]
#[case::normalized_template(("  color:\n    `${tone} !important`,", 4, 5, "`${tone}`"))]
#[serial]
fn live_dynamic_leaf_retains_authored_offset_when_object_tag_is_extracted(
    #[case] sample: (&str, usize, usize, &str),
    #[values(false, true)] single_css: bool,
) {
    // Given: line/column and normalized code are independent authored expectations.
    let _state = crate::test_state::TestStateGuard::new();
    let (rules, line, column, code) = sample;
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\nexport const card = css`${{{{\n{rules}\n}}}}`;"
    );
    // When
    let error = extract(
        "/src/policy.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_err()
    .to_string();
    // Then
    assert!(
        error.starts_with(&format!("/src/policy.ts:{line}:{column}: ")),
        "{error}"
    );
    assert!(
        error.contains(&format!("cannot use `{code}` at build time:")),
        "{error}"
    );
}

#[rstest]
#[case::spread("  ...{ color: tone }, color: 'red',")]
#[case::property("  ...{}, color: tone, color: 'red',")]
#[case::dead_branch("  color: false ? tone : 'red',")]
#[case::selected_member("  color: ({ dead: tone, live: 'red' })['live'],")]
#[serial]
fn static_rules_survive_when_dead_dynamic_values_are_discarded(
    #[case] rules: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    css::debug::set_debug(true);
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\nexport const card = css`${{{{\n{rules}\n}}}}`;"
    );
    // When
    let output = extract(
        "/src/policy.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap();
    // Then
    assert_composed_export_atoms(&output.code, "card", &["color-0-red--255"]);
}

#[rstest]
#[case::ordinary_call("css({ color: tone })")]
#[case::composed_call("css(css({ background: 'red' }), { color: tone })")]
#[case::scalar_tag("css`color: ${tone};`")]
#[case::scalar_tag_with_object("css`${{ background: 'red' }}; color: ${tone};`")]
#[serial]
fn default_caller_keeps_container_diagnostic_when_value_is_dynamic(#[case] expression: &str) {
    // Given: these caller offsets are the established pre-policy contract.
    let _state = crate::test_state::TestStateGuard::new();
    let source =
        format!("import {{ css }} from '@devup-ui/react';\nexport const card = {expression};");
    // When
    let error = extract("/src/default.ts", &source, ExtractOption::default())
        .unwrap_err()
        .to_string();
    // Then
    assert!(error.starts_with("/src/default.ts:2:21: "), "{error}");
    assert!(
        error.contains("cannot use `tone` at build time:"),
        "{error}"
    );
}

#[test]
#[serial]
fn enum_property_keeps_static_alternatives_when_object_tag_is_extracted() {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let source = "import { css } from '@devup-ui/react';
export const card = css`${{ positioning: mode }}`;";
    // When
    let output = extract("/src/enum.ts", source, ExtractOption::default()).unwrap();
    // Then
    assert_eq!(output.styles.len(), 4);
    assert!(
        output
            .styles
            .iter()
            .all(|style| matches!(style, ExtractStyleValue::Static(_)))
    );
}
