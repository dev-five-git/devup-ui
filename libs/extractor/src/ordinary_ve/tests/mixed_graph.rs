use rstest::rstest;
use serial_test::serial;

use super::mixed_review::emitted_predicate;
use super::mixed_support::{assert_consumed, extract};

#[rstest]
#[case(
    concat!("const shared=", "{value:7};export const {left,right}=(()=>{style({});return {left:shared,right:shared}})();"),
    "left===right && left===shared && left.value===7"
)]
#[case(
    concat!("const input=", "{child:{value:7}};export const result=(()=>{style({});return input.child})();"),
    "result===input.child && result.value===7"
)]
#[case(
    concat!("const input=", "{value:7};export const first=(()=>{style({});return input})();export const second=(()=>{style({});return input})();export default (()=>{style({});return input})();"),
    "first===second && first===input && __review_default__===input"
)]
#[case(
    "export const result=(()=>{style({});const data=Object.create(null);Object.defineProperty(data,'hidden',{value:7,writable:false,enumerable:false,configurable:false});return Object.preventExtensions(data)})();",
    "Object.getPrototypeOf(result)===null && !Object.isExtensible(result) && result.hidden===7 && !Object.getOwnPropertyDescriptor(result,'hidden').writable && !Object.getOwnPropertyDescriptor(result,'hidden').enumerable && !Object.getOwnPropertyDescriptor(result,'hidden').configurable"
)]
#[case(
    concat!("const input=", "{value:1};const before=input.value;export const result=(()=>{input.value=2;style({});return input})();const after=input.value;"),
    "before===1 && after===2 && input.value===2 && result===input"
)]
#[case(
    concat!("const input=", "{child:{value:1}};const old=input.child;export const result=(()=>{input.child={value:2};style({});return {old,current:input.child}})();"),
    "old.value===1 && result.old===old && result.current===input.child && input.child.value===2"
)]
#[case(
    concat!("export const first=", "(()=>{style({});return {value:7}})();export const second=(()=>{style({});return first})();"),
    "first===second && second.value===7"
)]
#[case(
    "const input={a:1,b:2};export const result=(()=>{delete input.a;input.a=1;style({});return input})();",
    "result===input && Object.keys(input).join(',')==='b,a'"
)]
#[case(
    "let input=1;const before=input;export const result=(()=>{input=2;style({});return input})();const after=input;",
    "before===1 && after===2 && input===2 && result===2"
)]
#[case(
    concat!("let input=", "{value:1};const before=input;export const result=(()=>{input={value:2};style({});return input})();"),
    "before.value===1 && before!==input && result===input && input.value===2"
)]
#[case(
    "const input={length:1,a:2,'01':3};export const result=(()=>{delete input.length;input.length=1;style({});return input})();",
    "result===input && Object.keys(input).join(',')==='a,01,length'"
)]
#[serial]
fn capture_graph_preserves_identity_and_state_when_roots_share_retained_data(
    #[case] body: &str,
    #[case] predicate: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';{body}const browser=window.document;"
    );
    // When
    let output = extract("ts", &source)?;
    // Then
    assert_consumed(&output);
    assert!(
        emitted_predicate(&output.code, predicate),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn capture_scalars_are_hygienic_when_authored_names_shadow_intrinsics()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';const Object='shadow',NaN='shadow',Infinity='shadow',undefined='shadow';export const result=(()=>{style({});return {negative:-0,nan:0/0,positive:1/0,negativeInfinity:-1/0,missing:void 0}})();const browser=window.document;";
    // When
    let output = extract("ts", source)?;
    // Then
    assert_consumed(&output);
    assert!(
        emitted_predicate(
            &output.code,
            "1/result.negative===-1/0 && result.nan!==result.nan && result.positive===1/0 && result.negativeInfinity===-1/0 && result.missing===void 0"
        ),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn required_unknown_read_stays_unknown_when_its_name_matches_the_observer() {
    // Given
    let source = concat!(
        "import {style} from '@vanilla-extract/css';export const box=style(",
        "{width:__ve_observe__});const browser=window.document;"
    );
    let offset = source
        .find("__ve_observe__")
        .unwrap_or_else(|| panic!("fixture read missing"));
    let expected = crate::locate("/mixed.ts", source, offset);
    // When
    let result = extract("ts", source);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("observer made a required unknown input available"))
        .to_string();
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("__ve_observe__"), "{error}");
    assert!(error.contains("build time"), "{error}");
}

#[test]
#[serial]
fn exotic_capture_is_rejected_when_its_prototype_is_replaced_with_null() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';export const result=(()=>{style({});return Object.setPrototypeOf(new Map(),null)})();const browser=window.document;";
    let offset = source
        .find("result=")
        .unwrap_or_else(|| panic!("fixture result missing"));
    let expected = crate::locate("/mixed.ts", source, offset);
    // When
    let result = extract("ts", source);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("erased prototype made exotic state look like plain data"))
        .to_string();
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("cannot be captured exactly"), "{error}");
}
