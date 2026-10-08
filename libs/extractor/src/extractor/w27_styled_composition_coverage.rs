use super::super::super::w27_props_rule_choices_tests::support::{active, declarations};
use crate::{ExtractOption, extract};
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn unreadable_selector_when_interpolation_follows_literal_prefix_is_located()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import { styled } from '@devup-ui/react'; export const Choice = styled.div`&${runtime} { color: red; }`;";
    // When
    let error = extract("unplaced-selector.tsx", source, ExtractOption::default())
        .err()
        .ok_or("unknown selector unexpectedly compiled")?
        .to_string();
    // Then
    assert!(error.starts_with("unplaced-selector.tsx:1:"), "{error}");
    assert!(error.contains("runtime"), "{error}");
    Ok(())
}

#[rstest]
#[case("0 && unknownRules()", "green")]
#[case("1 && { color: 'red' }", "red")]
#[case("'' || { color: 'blue' }", "blue")]
#[case("'yes' && { color: 'red' }", "red")]
#[case("0 ?? unknownRules()", "green")]
#[case("'' ?? unknownRules()", "green")]
#[case("({ color: 'red' }) && { color: 'blue' }", "blue")]
#[serial]
fn literal_choices_when_short_circuited_select_only_reachable_rules(
    #[case] choice: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{ styled }} from '@devup-ui/react'; export const Choice = styled.div({{ color: 'green' }}, p => {choice});"
    );
    // When
    let output = extract(
        "literal-choice.tsx",
        &source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )?;
    // Then
    assert_eq!(
        declarations(&active(&output, "{}")),
        [("color".into(), expected.into())]
    );
    Ok(())
}

#[test]
#[serial]
fn later_mixin_when_prior_context_has_closed_is_not_nested_in_prior_selector()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import { styled, css } from '@devup-ui/react'; const mixin = css({ color: 'red' }); export const Choice = styled.div`&:hover { color: blue; } ${mixin};`;";
    // When
    let output = extract(
        "closed-context.tsx",
        source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )?;
    // Then
    let mut keys = active(&output, "{}")
        .into_iter()
        .map(|value| match value {
            crate::ExtractStyleValue::Static(style) => (
                style.value().to_string(),
                style.selector().map(ToString::to_string),
            ),
            other => panic!("expected static atom, got {other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        [
            ("blue".into(), Some("&:hover".into())),
            ("red".into(), None)
        ]
    );
    Ok(())
}

#[rstest]
#[case("p => { notify(p); return 'external'; }")]
#[case("function(p) { notify(p); }")]
#[serial]
fn unreadable_callback_when_nested_reports_interpolation_location(
    #[case] callback: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{ styled }} from '@devup-ui/react'; export const Choice = styled.div`&:hover {{ ${{{callback}}}; }}`;"
    );
    // When
    let error = extract("callback-boundary.tsx", &source, ExtractOption::default())
        .err()
        .ok_or("unreadable nested callback unexpectedly compiled")?
        .to_string();
    // Then
    assert!(error.starts_with("callback-boundary.tsx:1:"), "{error}");
    assert!(error.contains("nested mixin"), "{error}");
    Ok(())
}
