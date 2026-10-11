use rstest::rstest;
use serial_test::serial;

use super::{TestResult, observe, option, reset, stylesheet};
use crate::extract_without_source_map;

#[derive(Clone, Copy, Debug)]
enum ExportForm {
    Named,
    Multiple,
    Default,
}

#[rstest]
#[case(("const value=(x)=>x;", "exports.invoke(13)===13"))]
#[case(("const value=[()=>11];", "exports.invoke[0]()===11"))]
#[case((
    "const value=(()=>{const o={};o.self=o;return o})();",
    "exports.invoke.self===exports.invoke"
))]
#[case(("const value=Symbol('x');", "typeof exports.invoke==='symbol'"))]
#[serial]
fn authored_aliases_survive_when_exported_values_are_nonserializable(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(ExportForm::Named, ExportForm::Multiple, ExportForm::Default)] form: ExportForm,
    #[case] fixture: (&str, &str),
) -> TestResult {
    // Given
    reset();
    let (declaration, predicate) = fixture;
    let (exports, identity) = match form {
        ExportForm::Named => ("export {value as invoke};", "true"),
        ExportForm::Multiple => (
            "export {value as invoke,value as again,value as default};",
            "exports.invoke===exports.again && exports.invoke===exports.default",
        ),
        ExportForm::Default => (
            "export default value; export {value as invoke};",
            "exports.invoke===exports.default",
        ),
    };
    let source = stylesheet("@devup-ui/react", &format!("{declaration}\n{exports}"));
    let path = format!("/authored-alias.{suffix}");
    // When
    let output = extract_without_source_map(&path, &source, option())?;
    // Then
    observe(&output, &format!("({predicate}) && ({identity})"))
}

#[rstest]
#[case(("export const value=(x)=>x;", "exports.value(13)===13"))]
#[case(("export const value=[()=>11];", "exports.value[0]()===11"))]
#[case((
    "export const value=(()=>{const o={};o.self=o;return o})();",
    "exports.value.self===exports.value"
))]
#[case(("export const value=Symbol('x');", "typeof exports.value==='symbol'"))]
#[serial]
fn authored_values_survive_when_only_styling_is_compiled(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values("@devup-ui/react", "@vanilla-extract/css")] package: &str,
    #[case] fixture: (&str, &str),
) -> TestResult {
    // Given
    reset();
    let (body, predicate) = fixture;
    let source = stylesheet(package, body);
    let path = format!("/authored-direct.{suffix}");
    // When
    let output = extract_without_source_map(&path, &source, option())?;
    // Then
    observe(&output, predicate)
}

#[rstest]
#[case(
    "export const value=(x)=>x; export {value as invoke};",
    "exports.value===exports.invoke && exports.invoke(13)===13"
)]
#[case(
    "function value(x){return x} export {value as invoke};",
    "exports.invoke(13)===13"
)]
#[case(
    "const {value}={value:(x)=>x}; export {value as invoke};",
    "exports.invoke(13)===13"
)]
#[case("export default (x)=>x;", "exports.default(13)===13")]
#[case(
    "const value=(x)=>x; export {value as 'call me'};",
    "exports['call me'](13)===13"
)]
#[serial]
fn authored_export_forms_survive_when_callable_bindings_are_not_plain_data(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] body: &str,
    #[case] predicate: &str,
) -> TestResult {
    // Given
    reset();
    let source = stylesheet("@devup-ui/react", body);
    let path = format!("/authored-forms.{suffix}");
    // When
    let output = extract_without_source_map(&path, &source, option())?;
    // Then
    observe(&output, predicate)
}

#[rstest]
#[case(
    "const seed=7; export const helper=(seed)=>seed;",
    "exports.helper(13)===13"
)]
#[case(
    "const seed=7; export const helper=()=>({seed:1,text:'seed'});",
    "exports.helper().seed===1 && exports.helper().text==='seed'"
)]
#[case(
    "const seed=7; export const helper=({seed})=>seed;",
    concat!("exports.helper({", "seed:13", "})===13")
)]
#[case(
    "const seed=7; export const helper=()=>{try{throw 13}catch(seed){return seed}};",
    "exports.helper()===13"
)]
#[serial]
fn lexical_names_remain_safe_when_fallbacks_do_not_read_removed_bindings(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] body: &str,
    #[case] predicate: &str,
) -> TestResult {
    // Given
    reset();
    let source = stylesheet("@devup-ui/react", body);
    let path = format!("/authored-shadow.{suffix}");
    // When
    let output = extract_without_source_map(&path, &source, option())?;
    // Then
    observe(&output, predicate)
}

#[rstest]
#[serial]
fn compiled_class_closure_survives_when_native_rewrite_retains_its_binding(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@vanilla-extract/css';\nconst box=style({color:'blue'});\nexport {box};\nexport const helper=()=>box;";
    let path = format!("/authored-native-class.{suffix}");
    // When
    let output = extract_without_source_map(&path, source, option())?;
    // Then
    observe(&output, "exports.helper()===exports.box")
}

#[rstest]
#[serial]
fn authored_helper_survives_when_compiled_global_effects_call_it(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = "import {style,globalStyle} from '@devup-ui/react';\nexport const helper=()=> 'purple';\nglobalStyle('body',{color:helper()});\nexport const box=style({color:'blue'});";
    // When
    let output =
        extract_without_source_map(&format!("/authored-global.{suffix}"), source, option())?;
    // Then
    assert!(output.styles.iter().any(|value| matches!(value,
        crate::ExtractStyleValue::Static(style) if style.property == "color" && style.value == "purple"
            && matches!(&style.selector, Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "body")
    )));
    observe(&output, "exports.helper()==='purple'")
}

#[rstest]
#[case("const value={};value.self=value;export {value as loop,value as again};")]
#[case("const value={};const alias=value;alias.self=alias;export {value as loop,value as again};")]
#[serial]
fn authored_mutation_survives_when_an_aliased_value_has_a_cycle_after_initialization(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] body: &str,
) -> TestResult {
    // Given
    reset();
    let source = stylesheet("@devup-ui/react", body);
    // When
    let output =
        extract_without_source_map(&format!("/authored-mutation.{suffix}"), &source, option())?;
    // Then
    observe(
        &output,
        "exports.loop.self===exports.loop && exports.loop===exports.again",
    )
}
