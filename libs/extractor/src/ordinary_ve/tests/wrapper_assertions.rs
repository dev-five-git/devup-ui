use rstest::rstest;
use serial_test::serial;

use super::wrapper_runtime::{exported_identity_and_class, validate};
use crate::extract_style::style_property::StyleProperty;
use crate::{ExtractOption, ExtractStyleValue, extract_with_modules};

#[rstest]
#[case("/wrapper-plain.css.ts", "(function(){value.self=value;})()")]
#[case(
    "/wrapper-as.css.ts",
    "((function(){value.self=value;}) as (()=>void))()"
)]
#[case(
    "/wrapper-assertion.css.ts",
    "(<(()=>void)>function(){value.self=value;})()"
)]
#[serial]
fn exported_cycle_and_blue_style_survive_when_iife_callee_is_wrapped(
    #[case] filename: &str,
    #[case] initializer: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    let source = format!(
        "import {{style}} from '@devup-ui/react';const value={{}};const setup={initializer};export {{value as loop}};export const box=style({{color:'blue'}});"
    );
    validate(filename, &source)?;
    // When
    let output = extract_with_modules(
        filename,
        &source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &|_, _| None,
    )?;
    // Then
    let (identity, classes) = exported_identity_and_class(filename, &output.code)?;
    assert!(
        identity,
        "compiled loop lost its self identity: {}",
        output.code
    );
    let active_colors = output
        .styles
        .iter()
        .filter_map(|value| {
            let ExtractStyleValue::Static(style) = value else {
                return None;
            };
            let Some(StyleProperty::ClassName(class)) = value.extract(None) else {
                return None;
            };
            (style.property() == "color" && classes.split_whitespace().any(|token| token == class))
                .then(|| style.value().to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(active_colors, vec!["blue"]);
    Ok(())
}
