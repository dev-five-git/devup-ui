use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, reset, run, static_value};

#[rstest]
#[case("const")]
#[case("let")]
#[serial]
fn coverage_demand_runtime_compiles_plain_bindings_and_exported_functions(
    #[case] keyword: &str,
) -> TestResult {
    // Given
    reset();
    let producer = format!(
        "import {{style}} from '@vanilla-extract/css';{keyword} base=style({{padding:8}});export function read(){{return base}}const browser=window.document;"
    );
    let source = "import {style,globalStyle} from '@vanilla-extract/css';import {read} from './producer';import {read as again} from './producer';export const box=style([read(),{ color:'blue' }]);globalStyle(`${again()}:focus`,{ outlineColor:'purple' });";
    // When
    let output = run(
        "/runtime.ts",
        source,
        &[("./producer", "/producer.ts", &producer)],
    )?;
    // Then
    assert_eq!(static_value(&output, "padding"), "8px");
    assert_eq!(static_value(&output, "color"), "blue");
    assert_eq!(
        super::demand_support::global_selector(&output, "outline-color"),
        ".f0_0:focus"
    );
    assert!(!output.code.contains("__style_"), "{}", output.code);
    Ok(())
}

#[test]
#[serial]
fn coverage_demand_runtime_finalizes_when_authored_initializer_catches_its_exception() -> TestResult
{
    // Given
    reset();
    let producer = "import {style} from '@vanilla-extract/css';export const base=(()=>{try{throw new Error('authored')}catch(error){return style({ padding:8 })}})();const browser=window.document;";
    let source = "import {style} from '@vanilla-extract/css';import {base} from './producer';export const box=style([base,{ color:'blue' }]);";
    // When
    let output = run(
        "/caught.ts",
        source,
        &[("./producer", "/caught-producer.ts", producer)],
    )?;
    // Then
    assert_eq!(static_value(&output, "padding"), "8px");
    assert_eq!(static_value(&output, "color"), "blue");
    Ok(())
}

#[test]
#[serial]
fn coverage_demand_runtime_reports_authored_throw_before_owner_validation() -> TestResult {
    // Given
    reset();
    let producer = "import {style} from '@vanilla-extract/css';\nexport const base=(()=>{throw new Error('authored initialization failed');return style({ padding:8 })})();const browser=window.document;";
    let place = crate::locate(
        "/throw-producer.ts",
        producer,
        producer.find("new Error").ok_or("error missing")?,
    );
    let source = "import {style} from '@vanilla-extract/css';import {base} from './producer';export const box=style([base,{ color:'blue' }]);";
    // When
    let result = run(
        "/throw.ts",
        source,
        &[("./producer", "/throw-producer.ts", producer)],
    );
    // Then
    located_failure(result, &place, "authored initialization failed");
    Ok(())
}

#[rstest]
#[case(
    "module.exports",
    "may be changed outside its selected initialization slice"
)]
#[case("module.exports.space", "success")]
#[serial]
fn coverage_demand_runtime_audits_real_commonjs_binding_values(
    #[case] escaped: &str,
    #[case] cause: &str,
) -> TestResult {
    // Given
    reset();
    let helper = format!("module.exports={{space:'8px'}};\nexternal({escaped});");
    let producer = "import {createTheme} from '@vanilla-extract/css';import {space} from './tokens';export const [theme,vars]=createTheme({space});";
    let source = "import {css} from '@devup-ui/react';import {vars} from './producer';export const box=css({margin:vars.space});";
    let place = crate::locate(
        "/audit-tokens.js",
        &helper,
        helper.rfind(escaped).ok_or("escape missing")?,
    );
    // When
    let result = run(
        "/audit.ts",
        source,
        &[
            ("./producer", "/audit-producer.ts", producer),
            ("./tokens", "/audit-tokens.js", &helper),
        ],
    );
    // Then
    if cause == "success" {
        assert_eq!(static_value(&result?, "margin"), "var(--space-0-1)");
    } else {
        located_failure(result, &place, cause);
    }
    Ok(())
}
