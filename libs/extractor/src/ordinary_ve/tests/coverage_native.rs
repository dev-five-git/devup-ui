use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::{assert_consumed, has_static};

#[rstest]
#[case("const key='style';const make=ve[key];export {make};")]
#[case("const {style:make=()=>{throw new Error('fallback must not run')}}=ve;export {make};")]
#[serial]
fn barrel_alias_emits_native_units_when_computed_or_default_binding_is_exact(
    #[case] alias: &str,
) -> TestResult {
    // Given
    reset();
    let barrel = format!(
        "import * as ve from '@vanilla-extract/css';{alias}throw new Error('barrel host only');"
    );
    let source = "import {make} from './api';export const box=make({color:'blue',padding:8});const browser=window.document;";
    // When
    let output = run(
        "/native-alias.ts",
        source,
        &[("./api", "/alias-api.ts", &barrel)],
    )?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert_import(&output, "./api");
    assert!(
        output
            .dependencies
            .iter()
            .any(|path| path == "/alias-api.ts")
    );
    Ok(())
}

#[rstest]
#[case(
    "const make=ve.style;make.extra=1;export {make};",
    "import {make} from './api';",
    "make.extra"
)]
#[case(
    "const native=ve;native.extra=1;export default native.style;",
    "import make from './api';",
    "native.extra"
)]
#[serial]
fn changed_export_fails_at_original_unicode_site_when_consumed_through_another_barrel(
    #[case] declaration: &str,
    #[case] import: &str,
    #[case] mutation: &str,
) -> TestResult {
    // Given
    reset();
    let changed =
        format!("const 한글='😀';\r\nimport * as ve from '@vanilla-extract/css';\r\n{declaration}");
    let place = crate::locate(
        "/changed-native.ts",
        &changed,
        changed.find(mutation).ok_or("mutation missing")?,
    );
    let source =
        format!("{import}export const box=make({{color:'stale'}});const browser=window.document;");
    let forward = if declaration.contains("export default") {
        "export {default} from './changed';"
    } else {
        "export {make} from './changed';"
    };
    let graph = [
        ("./api", "/forward-native.ts", forward),
        ("./changed", "/changed-native.ts", changed.as_str()),
    ];
    // When
    let result = run("/changed-entry.ts", &source, &graph);
    // Then
    located_failure(result, &place, "may be changed");
    Ok(())
}

#[rstest]
#[case("export {style as make} from './missing';")]
#[case("export * from './missing';")]
#[serial]
fn unreadable_native_reexport_fails_at_import_when_package_provenance_is_present(
    #[case] reexport: &str,
) -> TestResult {
    // Given
    reset();
    let barrel = format!("import '@vanilla-extract/css';{reexport}");
    let source = "import {make} from './api';\r\nexport const box=make({color:'stale'});const browser=window.document;";
    let place = crate::locate(
        "/unreadable-entry.ts",
        source,
        source.find("make}").ok_or("import missing")?,
    );
    // When
    let result = run(
        "/unreadable-entry.ts",
        source,
        &[("./api", "/unreadable-api.ts", &barrel)],
    );
    // Then
    located_failure(
        result,
        &place,
        "native re-export `./missing` cannot be read",
    );
    Ok(())
}

#[test]
#[serial]
fn absent_star_candidate_keeps_the_real_api_when_an_ordinary_sibling_exports_other_names()
-> TestResult {
    // Given
    reset();
    let graph = [
        (
            "./api",
            "/star-api.ts",
            "export * from './ordinary';export * from './terminal';",
        ),
        ("./ordinary", "/ordinary.ts", "export const other=13;"),
        (
            "./terminal",
            "/terminal.ts",
            "export {style as make} from '@vanilla-extract/css';",
        ),
    ];
    // When
    let output = run(
        "/star-entry.ts",
        concat!(
            "import {make} from './api';export const box=make({",
            "padding:8",
            "});const browser=window.document;"
        ),
        &graph,
    )?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    for dependency in ["/star-api.ts", "/ordinary.ts", "/terminal.ts"] {
        assert!(output.dependencies.iter().any(|path| path == dependency));
    }
    Ok(())
}

#[test]
#[serial]
fn namespace_member_emits_native_css_when_it_comes_from_a_resolved_project_module() -> TestResult {
    // Given
    reset();
    let graph = [
        (
            "./api",
            "/namespace-api.ts",
            "import * as terminal from './terminal';const key='style';export default terminal[key];",
        ),
        (
            "./terminal",
            "/namespace-terminal.ts",
            "export {style} from '@vanilla-extract/css';",
        ),
    ];
    // When
    let output = run(
        "/namespace-entry.ts",
        "import make from './api';export const box=make({color:'blue',padding:8});const browser=window.document;",
        &graph,
    )?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert_import(&output, "./api");
    for dependency in ["/namespace-api.ts", "/namespace-terminal.ts"] {
        assert!(output.dependencies.iter().any(|path| path == dependency));
    }
    Ok(())
}
