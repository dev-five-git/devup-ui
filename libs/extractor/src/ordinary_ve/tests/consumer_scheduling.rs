use serial_test::serial;

use super::demand_support::{TestResult, reset, run};
use super::mixed_support::has_static;
use crate::jsx_semantics_tests::whole::evaluate_compiled;

pub(super) const PRODUCER: &str = concat!(
    "import {createVar} from '@vanilla-extract/css';export const token=createVar();",
    "let calls=0;export function read(){calls++;return calls===1?'blue':'green'}",
    "export function other(){calls++;return 'red'}export function count(){return calls}",
    "const browser=window.document;"
);

#[test]
#[serial]
fn consumer_slots_when_native_unit_separates_them_preserve_helper_order() -> TestResult {
    // Given
    reset();
    let source = concat!(
        "import {css} from '@devup-ui/react';import {style} from '@vanilla-extract/css';",
        "import {token,read,count} from './producer';",
        "css({color:read(),margin:token});",
        "export const middle=style({padding:count()+'px'});",
        "css({background:read()});export const last=css({borderWidth:count()+'px'});"
    );
    // When
    let output = run(
        "/consumer-order.ts",
        source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"), "{:?}", output.styles);
    assert!(has_static(&output, "padding", "1px"), "{:?}", output.styles);
    assert!(
        has_static(&output, "background", "green"),
        "{:?}",
        output.styles
    );
    assert!(
        has_static(&output, "border-width", "2px"),
        "{:?}",
        output.styles
    );
    Ok(())
}

#[test]
#[serial]
fn consumer_capture_when_original_names_collide_preserves_values_and_calls() -> TestResult {
    // Given
    reset();
    let source = concat!(
        "import {css} from '@devup-ui/react';import {token,read,count} from './producer';",
        "export const __ve_consumer_0__='authored-first',__ve_consumer_0___='authored-second';",
        "export const box=css({color:read(),margin:token});",
        "export const check=css({padding:count()+'px'});"
    );
    // When
    let output = run(
        "/consumer-hygiene.ts",
        source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"), "{:?}", output.styles);
    assert!(has_static(&output, "padding", "1px"), "{:?}", output.styles);
    let actual = evaluate_compiled(&output.code, "", "[__ve_consumer_0__,__ve_consumer_0___]");
    assert_eq!(
        actual.element,
        serde_json::json!(["authored-first", "authored-second"])
    );
    Ok(())
}

#[test]
#[serial]
fn consumer_shorthand_when_readback_supplies_color_keeps_property_meaning() -> TestResult {
    // Given
    reset();
    let producer = "import {createVar} from '@vanilla-extract/css';export const token=createVar();export const color='blue';const browser=window.document;";
    let source = "import {css} from '@devup-ui/react';import {token,color} from './producer';export const box=css({margin:token,color});";
    // When
    let output = run(
        "/consumer-shorthand.ts",
        source,
        &[("./producer", "/shorthand-owner.ts", producer)],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"), "{:?}", output.styles);
    super::demand_support::assert_import(&output, "./producer");
    Ok(())
}
