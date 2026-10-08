use rstest::rstest;
use serial_test::serial;

use super::{TestResult, observe, option, reset};
use crate::{ExtractStyleValue, extract_without_source_map};

#[rstest]
#[case(
    "const {helper,n}={helper:(x)=>x,n:7};export {helper as invoke,n};",
    "exports.invoke(13)===13 && exports.n===7"
)]
#[case(
    "const value={};const setup=(value.self=value);export {value as loop,value as again};",
    "exports.loop.self===exports.loop && exports.loop===exports.again"
)]
#[case(
    concat!("const value={};const setup=Object.setPrototypeOf(value,{", "tag:7", "});export {value as loop,value as again};"),
    "Object.getPrototypeOf(exports.loop).tag===7 && exports.loop===exports.again"
)]
#[case(
    "var value=()=>1;export default value;var value=()=>2;export {value as current};",
    "exports.default()===1 && exports.current()===2 && exports.default!==exports.current"
)]
#[serial]
fn authored_export_state_is_preserved_when_declaration_units_share_or_change_bindings(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] body: &str,
    #[case] predicate: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@devup-ui/react';\n{body}\nexport const box=style({{color:'blue'}});"
    );
    // When
    let output =
        extract_without_source_map(&format!("/authored-oracle.{suffix}"), &source, option())?;
    // Then
    observe(&output, predicate)
}

fn assert_purple(output: &crate::ExtractOutput) {
    assert!(output.styles.iter().any(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property == "color" && style.value == "purple"
            && matches!(&style.selector, Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "body")
    )));
}

#[rstest]
#[serial]
fn compiled_effects_preserve_live_function_assignment_or_refuse_its_original_site(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let path = format!("/authored-live-write.{suffix}");
    let source = "import {style,globalStyle} from '@devup-ui/react';\nexport let helper=()=>1;\nglobalStyle((helper=()=>2,'body'),{color:'purple'});\nexport const box=style({color:'blue'});";
    // When
    let result = extract_without_source_map(&path, source, option());
    // Then
    match result {
        Ok(output) => {
            assert_purple(&output);
            observe(&output, "exports.helper()===2")
        }
        Err(error) => {
            let error = error.to_string();
            assert!(error.starts_with(&format!("{path}:3:1:")), "{error}");
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

#[rstest]
#[serial]
fn compiled_only_helper_results_are_not_retained_state_aliases(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = "import {style,globalStyle} from '@devup-ui/react';\nexport const helper=()=> 'purple';\nconst color=helper();\nglobalStyle('body',{color});\nexport const box=style({color:'blue'});";
    // When
    let output = extract_without_source_map(
        &format!("/authored-compiled-input.{suffix}"),
        source,
        option(),
    )?;
    // Then
    assert_purple(&output);
    observe(&output, "exports.helper()==='purple'")
}
