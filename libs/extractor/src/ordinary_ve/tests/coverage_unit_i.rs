use serial_test::serial;

use super::demand_support::{TestResult, reset, run, static_value};

#[test]
#[serial]
fn coverage_unit_i_consumer_readback_materializes_selected_helper_value() -> TestResult {
    // Given
    reset();
    let source = "import {css} from '@devup-ui/react';import {space} from './producer';export const box=css({margin:space()});const browser=window.document;";
    let producer = "import {style} from '@vanilla-extract/css';const base=style({ padding:8 });export function space(){return base.length+'px'}const browser=window.document;";
    // When
    let output = run(
        "/readback.ts",
        source,
        &[("./producer", "/producer.ts", producer)],
    )?;
    // Then
    assert_eq!(static_value(&output, "margin"), "6px");
    assert!(output.code.contains("window.document"), "{}", output.code);
    assert!(!output.code.contains("margin: space()"), "{}", output.code);
    Ok(())
}

#[test]
#[serial]
fn coverage_unit_i_namespace_alias_preserves_native_identity() -> TestResult {
    // Given
    reset();
    let source = "import {make} from './alias';export const box=make({color:'blue'});const browser=window.document;";
    let alias = "import * as other from './api';const make=other.style;export {make};";
    let api = "export {style} from '@vanilla-extract/css';";
    // When
    let output = run(
        "/alias.ts",
        source,
        &[
            ("./alias", "/alias-barrel.ts", alias),
            ("./api", "/api.ts", api),
        ],
    )?;
    // Then
    assert_eq!(static_value(&output, "color"), "blue");
    assert!(output.code.contains("window.document"), "{}", output.code);
    assert!(!output.code.contains("make("), "{}", output.code);
    Ok(())
}
