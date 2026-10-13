use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, reset, run, static_value};
use super::mixed_review::emitted_predicate;

#[rstest]
#[case(
    "{a:1,b:2}",
    "null",
    "Object.getPrototypeOf(input)===null && result===input && result.a===3 && result.b===2"
)]
#[case(
    "Object.assign(Object.create(null),{a:1,b:2})",
    "Object.prototype",
    "Object.getPrototypeOf(input)===Object.prototype && result===input && result.a===3 && result.b===2"
)]
#[case(
    "[1,2]",
    "null",
    "Array.isArray(input) && Object.getPrototypeOf(input)===null && result===input && result[0]===3 && result[1]===2 && result.length===2"
)]
#[case(
    "Object.setPrototypeOf([1,2],null)",
    "Array.prototype",
    "Array.isArray(input) && Object.getPrototypeOf(input)===Array.prototype && result===input && result[0]===3 && result[1]===2 && result.length===2"
)]
#[serial]
fn coverage_emission_preserves_prototype_updates_and_unchanged_properties(
    #[case] initial: &str,
    #[case] prototype: &str,
    #[case] predicate: &str,
) -> TestResult {
    reset();
    let write = if initial.contains('[') {
        "input[0]=3;"
    } else {
        "input.a=3;"
    };
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';const input={initial};export const result=(()=>{{Object.setPrototypeOf(input,{prototype});{write}style({{padding:7}});return input}})();const browser=window.document;"
    );
    let output = run("/emission.css.ts", &source, &[])?;
    assert_eq!(static_value(&output, "padding"), "7px");
    assert!(
        emitted_predicate(&output.code, predicate),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn coverage_emission_preserves_an_immutable_property_when_other_keys_move() -> TestResult {
    reset();
    let source = "import {style} from '@vanilla-extract/css';\nconst input=(()=>{const data={ a:1 };Object.defineProperty(data,'b',{value:2,enumerable:true});data.c=3;return data})();\nexport const result=(()=>{delete input.a;delete input.c;input.c=3;input.a=1;style({padding:7});return input})();const browser=window.document;";
    let output = run("/immutable.css.ts", source, &[])?;
    assert_eq!(static_value(&output, "padding"), "7px");
    assert!(
        emitted_predicate(
            &output.code,
            "result===input && Object.keys(result).join(',')==='b,c,a' && result.a===1 && result.b===2 && result.c===3 && !Object.getOwnPropertyDescriptor(result,'b').configurable && !Object.getOwnPropertyDescriptor(result,'b').writable"
        ),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn coverage_emission_avoids_reserved_object_and_node_names() -> TestResult {
    reset();
    let source = "import {style} from '@vanilla-extract/css';const __ve_object__='object',__ve_object___='second',__ve_node_0__='node';export const result=(()=>{style({padding:9});return Object.assign(Object.create(null),{value:7})})();const browser=window.document;";
    let output = run("/names.css.ts", source, &[])?;
    assert_eq!(static_value(&output, "padding"), "9px");
    assert!(
        emitted_predicate(
            &output.code,
            "Object.getPrototypeOf(result)===null && result.value===7 && __ve_object__==='object' && __ve_object___==='second' && __ve_node_0__==='node'"
        ),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case("Object.assign(Object.create(null),{color:'blue'})")]
#[case("(()=>{const shared={color:'blue'};return {a:shared,b:shared}})()")]
#[serial]
fn coverage_emission_rejects_nonliteral_consumer_objects(#[case] value: &str) -> TestResult {
    reset();
    let producer = format!(
        "import {{style}} from '@vanilla-extract/css';export const data=(()=>{{style({{padding:1}});return {value}}})();const browser=window.document;"
    );
    let source = "import {css} from '@devup-ui/react';import {data} from './producer';\nexport const box=css(data);";
    let place = crate::locate(
        "/consumer.css.ts",
        source,
        source.rfind("data").ok_or("read missing")?,
    );
    let result = run(
        "/consumer.css.ts",
        source,
        &[("./producer", "/producer.css.ts", &producer)],
    );
    located_failure(
        result,
        &place,
        "consumer result cannot be represented exactly as a styling literal",
    );
    Ok(())
}

#[rstest]
#[case("({get color(){return 'red'}})", "accessor property")]
#[case(
    "new (class Color { constructor(){this.color='red'} })()",
    "exotic object"
)]
#[case("({[Symbol('color')]:'red'})", "symbol property")]
#[case("Symbol('color')", "symbol or unsupported scalar")]
#[serial]
fn coverage_emission_locates_unsupported_native_results(
    #[case] value: &str,
    #[case] cause: &str,
) -> TestResult {
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';\nexport const result=(()=>{{style({{padding:1}});return {value}}})();const browser=window.document;"
    );
    let place = crate::locate(
        "/unsupported.css.ts",
        &source,
        source.find("result=").ok_or("result missing")?,
    );
    let result = run("/unsupported.css.ts", &source, &[]);
    located_failure(
        result,
        &place,
        &format!("cannot be captured exactly: {cause}"),
    );
    Ok(())
}
