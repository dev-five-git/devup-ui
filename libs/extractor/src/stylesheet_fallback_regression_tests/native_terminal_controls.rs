use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(("export const helper=()=>13;", "exports.helper()===13"))]
#[case(("export const helpers=[()=>13];", "exports.helpers[0]()===13"))]
#[case(("export const loop=(()=>{const value={};value.self=value;return value})();", "exports.loop.self===exports.loop"))]
#[case(("export const token=Symbol('native sibling');", "typeof exports.token==='symbol' && exports.token.description==='native sibling'"))]
#[case(("export const date=new Date(0);", "exports.date.getTime()===0"))]
#[serial]
fn ordinary_sibling_when_native_stylesheet_exports_no_api_keeps_authored_runtime_value(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] fixture: (&str, &str),
) -> TestResult {
    // Given
    reset();
    let (body, predicate) = fixture;
    let source = stylesheet("@vanilla-extract/css", body);
    // When
    let output = crate::extract_with_modules(
        &format!("/b-native-sibling.{suffix}"),
        &source,
        option(),
        false,
        &|_, _| None,
    )?;
    // Then: execute the emitted exports, not just retained spellings.
    observe(&output, predicate)
}
