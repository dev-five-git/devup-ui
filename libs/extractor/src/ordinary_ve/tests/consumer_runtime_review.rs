use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, reset, run};
use super::mixed_support::has_static;

#[rstest]
#[case(concat!("function color(){", "return window.name;}"))]
#[case("const color=()=>window.name;")]
#[serial]
fn consumer_native_leaf_is_static_when_an_unrelated_box_helper_needs_the_page(
    #[case] declaration: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{Box}} from '@devup-ui/react';import {{vars}} from './producer';{declaration}export const view=<Box m={{vars.space}} color={{color()}}/>;"
    );
    let producer = "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});";
    // When
    let output = run(
        "/runtime-helper.tsx",
        &source,
        &[("./producer", "/runtime-helper-producer.ts", producer)],
    )?;
    // Then
    assert!(has_static(&output, "margin", "var(--space-0-1)"));
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    assert!(output.code.contains("window.name"), "{}", output.code);
    assert!(output.code.contains("color()"), "{}", output.code);
    assert!(output.styles.iter().any(|value| matches!(value, crate::ExtractStyleValue::Dynamic(style) if style.property() == "color" && style.identifier().contains("color()"))));
    Ok(())
}
