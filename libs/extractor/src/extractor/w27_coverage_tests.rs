use crate::{ExtractOption, ExtractStyleValue, ResolvedModule, extract, extract_with_modules};
use rstest::rstest;
use serial_test::serial;

#[path = "w27_styled_composition_coverage.rs"]
mod styled_composition;

#[test]
fn declaration_oracle_when_given_unsupported_atom_rejects_instead_of_filtering_it() {
    // Given
    let styles = [ExtractStyleValue::Typography("body".into())];
    // When
    let result = std::panic::catch_unwind(|| {
        super::super::w27_props_rule_choices_tests::support::declarations(&styles)
    });
    // Then
    assert!(result.is_err(), "unsupported atoms must fail the oracle");
}

#[test]
#[serial]
fn active_oracle_when_output_has_unsupported_atom_rejects_even_if_class_is_inactive()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let mut output = extract(
        "oracle.tsx",
        "import { styled } from '@devup-ui/react'; export const Choice = styled.div`color: red;`;",
        super::option(),
    )?;
    output
        .styles
        .insert(ExtractStyleValue::Typography("body".into()));
    // When
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        super::super::w27_props_rule_choices_tests::support::active(&output, "{}")
    }));
    // Then
    assert!(
        result.is_err(),
        "inactive unsupported atoms must fail the oracle"
    );
    Ok(())
}

#[test]
#[serial]
fn static_declaration_oracle_when_output_contains_runtime_values_rejects_it()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import styled from '@emotion/styled'; export const Choice = styled.div`width: ${p => p.width};`;";
    let output = extract("coverage.tsx", source, super::option())?;
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
    );
    // When
    let result = super::checked_declarations(&output);
    // Then
    assert!(
        result.is_err(),
        "runtime atoms must not be discarded by the static oracle"
    );
    Ok(())
}

#[rstest]
#[case("runtime")]
#[case("(runtime as string)")]
#[serial]
fn empty_known_mixin_when_combined_with_unknown_class_preserves_the_class(
    #[case] class: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{ css }} from '@devup-ui/react'; export const result = css`${{css({{}})}}${{{class}}}`;"
    );
    // When
    let output = extract("coverage.tsx", &source, ExtractOption::default())?;
    // Then
    assert_eq!(output.styles.len(), 0);
    assert!(output.code.contains("runtime"), "{}", output.code);
    assert!(!output.code.contains("css`"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case("const rules = { color: props.color }; return css(...rules);")]
#[case(
    "const rules = { color: props.color }; expose(rules); return css([rules, { color: 'blue' }]);"
)]
#[case(
    "const rules = { color: props.color }; return <div css={{ [props.key]: 'red', ...rules }} />;"
)]
#[serial]
fn local_runtime_rules_when_shape_cannot_be_composed_report_the_original_location(
    #[case] body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source =
        format!("import {{ css }} from '@emotion/react'; export function App(props) {{ {body} }}");
    // When
    let error = extract("coverage-local.tsx", &source, super::option())
        .err()
        .ok_or("unknown local composition unexpectedly compiled")?
        .to_string();
    // Then
    assert!(error.starts_with("coverage-local.tsx:1:"), "{error}");
    assert!(error.contains("build time"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn deep_namespace_when_given_css_prop_does_not_inherit_one_level_definition()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "/** @jsxImportSource @emotion/react */ import * as UI from './base'; export const result = <UI.Nested.Child css={{ color: 'blue' }} />;";
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./base").then(|| ResolvedModule {
        path: "/w27/base.tsx".into(),
        code: "import styled from '@emotion/styled'; export const Child = styled.div`color: red;`;".into(),
    })
    };
    // When
    let output = extract_with_modules("/w27/entry.tsx", source, super::option(), false, &resolver)?;
    // Then
    assert!(output.code.contains("UI.Nested.Child"), "{}", output.code);
    assert_eq!(
        super::checked_declarations(&output)?,
        [("color".into(), "blue".into())]
    );
    Ok(())
}

#[rstest]
#[case("getUI().Child")]
#[case("UI.Nested.Child")]
#[case("UI[key]")]
#[serial]
fn unknown_namespace_selector_when_not_an_exact_member_reports_error(
    #[case] selector: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import styled from '@emotion/styled'; import * as UI from './base'; export const result = styled.div`${{{selector}}} {{ color: blue; }}`;"
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./base").then(|| ResolvedModule {
        path: "/w27/base.tsx".into(),
        code: "import styled from '@emotion/styled'; export const Child = styled.div`color: red;`;".into(),
    })
    };
    // When
    let error = extract_with_modules("/w27/entry.tsx", &source, super::option(), false, &resolver)
        .err()
        .ok_or("unknown selector unexpectedly compiled")?
        .to_string();
    // Then
    assert!(error.starts_with("/w27/entry.tsx:1:"), "{error}");
    assert!(error.contains(selector), "{error}");
    assert!(error.contains("at build time"), "{error}");
    Ok(())
}
