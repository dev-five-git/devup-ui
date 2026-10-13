use rstest::rstest;
use serial_test::serial;

use super::{TestResult, observe, option, reset};
use crate::{ExtractStyleValue, extract_without_source_map};

#[rstest]
#[case("function css(x){return x}export {css as invoke};")]
#[case("function globalCss(x){return x}export {globalCss as invoke};")]
#[case("function keyframes(x){return x}export {keyframes as invoke};")]
#[case(
    "function _ve0(x){return x}export {_ve0 as invoke};export const list=[style({color:'green'})];"
)]
#[serial]
fn authored_functions_keep_their_identity_when_generated_styling_names_need_hygiene(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] body: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style,globalStyle,keyframes as frames}} from '@devup-ui/react';\n{body}\nglobalStyle('body',{{color:'purple'}});\nconst spin=frames({{to:{{opacity:1}}}});\nexport const animated=style({{animationName:spin}});\nexport const box=style({{color:'blue'}});"
    );
    // When
    let output =
        extract_without_source_map(&format!("/authored-hygiene.{suffix}"), &source, option())?;
    // Then
    assert!(output.styles.iter().any(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property == "color" && style.value == "purple"
            && matches!(&style.selector, Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "body")
    )));
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|value| matches!(value, ExtractStyleValue::Keyframes(_)))
            .count(),
        1
    );
    observe(&output, "exports.invoke(13)===13")
}
