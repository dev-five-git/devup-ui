use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, reset, run, static_value};
use super::mixed_support::has_static;
use crate::extract_style::style_property::StyleProperty;
use crate::jsx_semantics_tests::whole::evaluate_compiled;

#[rstest]
#[case("", "blue")]
#[case("color='green'", "green")]
#[serial]
fn consumer_spread_when_exact_props_are_overridden_preserves_values_and_handler(
    #[case] override_prop: &str,
    #[case] color: &str,
) -> TestResult {
    // Given
    reset();
    let producer = "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const props={m:vars.space,color:'blue'};const browser=window.document;";
    let control = run(
        "/spread-owner.ts",
        "import {createTheme,style} from '@vanilla-extract/css';const [theme,vars]=createTheme({space:'8px'});export const check=style({margin:vars.space});",
        &[],
    )?;
    let margin = static_value(&control, "margin").to_string();
    let source = format!(
        "import {{Box}} from '@devup-ui/react';import {{props}} from './producer';export const view=<Box {{...props}} {override_prop} id='spread-view' onClick={{()=>document.title}}/>;"
    );
    // When
    let output = run(
        "/consumer-spread.tsx",
        &source,
        &[("./producer", "/spread-owner.ts", producer)],
    )?;
    // Then
    assert!(
        has_static(&output, "margin", &margin),
        "{:?}",
        output.styles
    );
    assert!(has_static(&output, "color", color), "{:?}", output.styles);
    let actual = evaluate_compiled(
        &output.code,
        "let calls=0;const document={get title(){calls++;return 'handler-result'}};",
        "(()=>{const before=calls;const value=view.props.onClick();return [[view.type,view.props.id,typeof view.props.onClick,before,value,calls],view.props.className]})()",
    );
    assert_eq!(
        actual.element[0],
        serde_json::json!(["div", "spread-view", "function", 0, "handler-result", 1])
    );
    let classes = actual.element[1]
        .as_str()
        .ok_or("element className missing")?;
    let applied_colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| match value {
            crate::ExtractStyleValue::Static(style) if style.property == "color" => {
                match value.extract(None) {
                    Some(StyleProperty::ClassName(class))
                        if classes.split_whitespace().any(|applied| applied == class) =>
                    {
                        Some(style.value.as_str())
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(applied_colors, [color]);
    Ok(())
}
