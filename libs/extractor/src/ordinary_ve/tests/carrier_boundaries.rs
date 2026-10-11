use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, reset, run};

#[rstest]
#[case(("const make=ve.style;make.extra=1;export {make};", "export {make} from './origin';", "import {make} from './api';", 21))]
#[case(("const native=ve;native.extra=1;export default native.style;", "export {default} from './origin';", "import make from './api';", 17))]
#[serial]
fn carrier_mutations_keep_original_unicode_crlf_columns_when_stylesheets_forward_them(
    #[values("tsx", "css.ts", "css.js")] suffix: &str,
    #[case] mutation: (&str, &str, &str, usize),
) {
    // Given
    reset();
    let (declaration, forward, import, column) = mutation;
    let path = format!("/changed-carrier.{suffix}");
    let producer =
        format!("const 한글='😀';\r\nimport * as ve from '@vanilla-extract/css';\r\n{declaration}");
    let source =
        format!("{import}export const box=make({{color:'stale'}});const browser=window.document;");
    let graph = [
        ("./api", "/changed-forward.css.ts", forward),
        ("./origin", path.as_str(), producer.as_str()),
    ];
    // When
    let result = run("/changed-carrier-entry.tsx", &source, &graph);
    // Then
    located_failure(result, &format!("{path}:3:{column}:"), "may be changed");
}

#[rstest]
#[case(("import * as ve from '@vanilla-extract/css';\r\nconst make=ve.style;export {make};", "ve.style"))]
#[case(("import * as ve from '@vanilla-extract/css';\r\nexport default ve.style;", "ve.style"))]
#[case(("import {make} from './terminal';\r\nexport {make};", "make};"))]
#[serial]
fn standalone_native_alias_or_bound_reexport_is_refused_when_no_internal_demand_exists(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] fixture: (&str, &str),
) -> TestResult {
    // Given
    reset();
    let (source, site) = fixture;
    let path = format!("/standalone-carrier.{suffix}");
    let place = crate::locate(&path, source, source.find(site).ok_or("escape missing")?);
    let graph = [(
        "./terminal",
        "/standalone-terminal.ts",
        "export {style as make} from '@vanilla-extract/css';",
    )];
    // When
    let result = run(&path, source, &graph);
    // Then
    let error = result
        .err()
        .ok_or("standalone native export succeeded")?
        .to_string();
    assert!(error.contains(&place), "{error}");
    assert!(
        error.contains("a native styling API escapes its exact initialization slice"),
        "{error}"
    );
    assert!(
        error.contains(
            "Fix: call the API in an exact initializer instead of exporting or handing off the API"
        ),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case(("import {make} from './api';", "export function render(){return make({color:'red'});}", "render", "a native styling helper remains reachable at runtime", "call the helper only from exact initializers and export the computed result"))]
#[case(("import * as api from './api';", "export const escaped=api;", "api;", "a native styling API escapes its exact initialization slice", "call the API in an exact initializer instead of exporting or handing off the API"))]
#[case(("import * as api from './api';", "const key=window.name;export const box=api[key]({color:'red'});", "api[key]", "a native styling namespace needs a static API member", "select the API with a static name or literal key"))]
#[serial]
fn unproved_runtime_helper_or_namespace_is_refused_when_a_stylesheet_carries_native_apis(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] fixture: (&str, &str, &str, &str, &str),
) -> TestResult {
    // Given
    reset();
    let (import, body, site, cause, fix) = fixture;
    let producer_path = format!("/unproved-api.{suffix}");
    let source = format!("const 한글='😀';\r\n{import}\r\n{body}const browser=window.document;");
    let place = crate::locate(
        "/unproved-entry.tsx",
        &source,
        source.find(site).ok_or("escape missing")?,
    );
    let graph = [(
        "./api",
        producer_path.as_str(),
        "export {style as make} from '@vanilla-extract/css';",
    )];
    // When
    let result = run("/unproved-entry.tsx", &source, &graph);
    // Then
    let error = result
        .err()
        .ok_or("unproved native escape succeeded")?
        .to_string();
    assert!(error.contains(&place), "{error}");
    assert!(error.contains(cause), "{error}");
    assert!(error.contains(&format!("Fix: {fix}")), "{error}");
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn required_window_input_is_located_at_original_owner_when_carrier_initialization_needs_it(
    #[values("tsx", "css.ts", "css.js")] consumer_suffix: &str,
    #[values("css.ts", "css.js")] producer_suffix: &str,
    #[case] producer_unknown: bool,
) -> TestResult {
    // Given
    reset();
    let producer_path = format!("/unknown-carrier.{producer_suffix}");
    let consumer_path = format!("/unknown-entry.{consumer_suffix}");
    let root = if producer_unknown {
        "const root=ve.style({color:window.name});"
    } else {
        ""
    };
    let producer = format!(
        "const 한글='😀';\r\nimport * as ve from '@vanilla-extract/css';\r\n{root}const key='style';const make=ve[key];export {{make}};const browser=window.document;"
    );
    let color = if producer_unknown {
        "'blue'"
    } else {
        "window.name"
    };
    let source = format!(
        "const 한글='😀';\r\nimport {{make}} from './api';\r\nexport const box=make({{color:{color}}});const browser=window.document;"
    );
    let (owner, original) = if producer_unknown {
        (producer_path.as_str(), producer.as_str())
    } else {
        (consumer_path.as_str(), source.as_str())
    };
    let place = crate::locate(
        owner,
        original,
        original.find("window.name").ok_or("unknown missing")?,
    );
    let graph = [("./api", producer_path.as_str(), producer.as_str())];
    // When
    let result = run(&consumer_path, &source, &graph);
    // Then
    located_failure(result, &place, "cannot use `window.name` at build time");
    Ok(())
}
