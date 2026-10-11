use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::{assert_consumed, has_static};

#[rstest]
#[case("const key='style';const make=ve[key];export {make};")]
#[case("const {style:make=()=>{throw new Error('fallback must not run')}}=ve;export {make};")]
#[serial]
fn exact_alias_is_carried_when_producer_and_consumer_suffixes_vary_independently(
    #[values("tsx", "css.ts", "css.js")] consumer_suffix: &str,
    #[values("tsx", "css.ts", "css.js")] producer_suffix: &str,
    #[case] alias: &str,
) -> TestResult {
    // Given
    reset();
    let producer_path = format!("/carrier-origin.{producer_suffix}");
    let producer = format!(
        "const 한글='😀';\r\nimport * as ve from '@vanilla-extract/css';\r\n{alias}const browser=window.document;throw new Error('producer runtime only');"
    );
    let source = "import {make} from './api';import {handler} from './runtime';export const box=make({color:'blue',padding:8});export const onClick=handler;const browser=window.document;";
    let graph = [
        (
            "./api",
            "/carrier-forward.ts",
            "export {make} from './origin';",
        ),
        ("./origin", producer_path.as_str(), producer.as_str()),
        (
            "./runtime",
            "/carrier-runtime.ts",
            "export const handler=()=>document.title;throw new Error('runtime dependency only');",
        ),
    ];
    // When
    let output = run(&format!("/carrier-entry.{consumer_suffix}"), source, &graph)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert!(!output.code.contains("make("), "{}", output.code);
    assert!(output.code.contains("window.document"), "{}", output.code);
    assert!(output.code.contains("handler"), "{}", output.code);
    for import in ["./api", "./runtime"] {
        assert_import(&output, import);
    }
    for dependency in ["/carrier-forward.ts", producer_path.as_str()] {
        assert!(output.dependencies.iter().any(|path| path == dependency));
    }
    Ok(())
}

#[rstest]
#[case(("export {make as renamed} from './origin';", "import {renamed as make} from './api';", "make"))]
#[case(("export {make as default} from './origin';", "import make from './api';", "make"))]
#[case(("export * from './origin';", "import {make} from './api';", "make"))]
#[case(("export * as api from './origin';", "import {api} from './api';", "api.make"))]
#[case(("export * from './origin';", "import * as api from './api';", "api.make"))]
#[serial]
fn pure_stylesheet_forwarder_carries_exact_calls_when_it_has_no_local_import(
    #[values("tsx", "css.ts", "css.js")] producer_suffix: &str,
    #[values("css.ts", "css.js")] forwarder_suffix: &str,
    #[case] forwarding: (&str, &str, &str),
) -> TestResult {
    // Given
    reset();
    let (forwarder, import, callee) = forwarding;
    let producer_path = format!("/forwarded-origin.{producer_suffix}");
    let forwarder_path = format!("/pure-forwarder.{forwarder_suffix}");
    let producer = "import * as ve from '@vanilla-extract/css';const key='style';const make=ve[key];export {make};const browser=window.document;";
    let graph = [
        ("./api", forwarder_path.as_str(), forwarder),
        ("./origin", producer_path.as_str(), producer),
    ];
    let source = format!(
        "{import}export const box={callee}({{color:'blue',padding:8}});const browser=window.document;"
    );
    // When
    let output = run("/forwarded-entry.tsx", &source, &graph)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert!(
        !output.code.contains(&format!("{callee}(")),
        "{}",
        output.code
    );
    assert!(output.code.contains("window.document"));
    assert_import(&output, "./api");
    for dependency in [forwarder_path.as_str(), producer_path.as_str()] {
        assert!(output.dependencies.iter().any(|path| path == dependency));
    }
    Ok(())
}

#[rstest]
#[serial]
fn default_terminal_crosses_two_pure_stylesheets_when_both_forward_default(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let terminal_path = format!("/default-origin.{suffix}");
    let middle_path = format!("/default-middle.{suffix}");
    let forward_path = format!("/default-forward.{suffix}");
    let graph = [
        (
            "./api",
            forward_path.as_str(),
            "export {default} from './middle';",
        ),
        (
            "./middle",
            middle_path.as_str(),
            "export {default} from './origin';",
        ),
        (
            "./origin",
            terminal_path.as_str(),
            "import * as ve from '@vanilla-extract/css';export default ve.style;",
        ),
    ];
    // When
    let output = run(
        "/default-entry.tsx",
        "import make from './api';export const box=make({color:'blue',padding:8});const browser=window.document;",
        &graph,
    )?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(!output.code.contains("make("));
    assert_import(&output, "./api");
    for dependency in [terminal_path, middle_path, forward_path] {
        assert!(output.dependencies.iter().any(|path| path == &dependency));
    }
    Ok(())
}
