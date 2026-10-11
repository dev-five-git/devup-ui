use rstest::rstest;
use serial_test::serial;

use super::{TestResult, observe, option, reset};
use crate::extract_without_source_map;

#[rstest]
#[case(
    "const value={};\nexport {value as loop};\nglobalStyle((value.self=value,'body'),{color:'purple'});",
    4
)]
#[case(
    "const value={};\nexport {value as loop};\nexport const helper=()=>{value.self=value;return 'purple'};\nglobalStyle('body',{color:helper()});",
    5
)]
#[serial]
fn compiled_effects_never_drop_retained_object_changes_when_their_arguments_mutate_it(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] body: &str,
    #[case] line: usize,
) -> TestResult {
    // Given
    reset();
    let path = format!("/authored-effect-state.{suffix}");
    let source = format!(
        "import {{style,globalStyle}} from '@devup-ui/react';\n{body}\nexport const box=style({{color:'blue'}});"
    );
    // When
    let result = extract_without_source_map(&path, &source, option());
    // Then: an unavailable rewrite is located, never a successful altered cyclic value.
    match result {
        Ok(output) => {
            assert!(output.styles.iter().any(|value| matches!(value,
                crate::ExtractStyleValue::Static(style) if style.property == "color" && style.value == "purple"
                    && matches!(&style.selector, Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "body")
            )));
            observe(&output, "exports.loop.self===exports.loop")
        }
        Err(error) => {
            let error = error.to_string();
            assert!(error.starts_with(&format!("{path}:{line}:1:")), "{error}");
            assert!(
                error.contains("`globalStyle`")
                    && error.contains("removed by stylesheet compilation"),
                "{error}"
            );
            assert!(error.contains("Fix:"), "{error}");
            Ok(())
        }
    }
}
