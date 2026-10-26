use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(false, SourceMapMode::Generate)]
#[case(false, SourceMapMode::Skip)]
#[case(true, SourceMapMode::Generate)]
#[case(true, SourceMapMode::Skip)]
#[serial]
fn deferred_records_survive_native_output(
    #[case] forwarded: bool,
    #[case] source_map: SourceMapMode,
) -> Result<(), String> {
    // Given an actual extraction with direct or forwarded render-only demand.
    let _state = TestStateGuard::new();
    let request = if forwarded {
        "./forward.js"
    } else {
        "./doc.mdx"
    };
    let source =
        format!("const label='한😀';\rimport Doc from '{request}'; export const view=<Doc/>;");
    let resolver = |request: &str, _: &str| {
        (request == "./forward.js").then(|| {
            ModuleResolution::Resolved(ResolvedModule {
                path: "/forward.js".to_string(),
                code: "export {default} from './doc.mdx'".to_string(),
                source_type: None,
            })
        })
    };
    // When either extraction mode constructs the real binding Output.
    let output = code_extract_internal_impl(
        "/page.tsx",
        &source,
        "@devup-ui/react",
        "df".to_string(),
        false,
        false,
        false,
        HashMap::new(),
        source_map,
        Some(&resolver),
        None,
        None,
        &[".mdx".to_string()],
    )?;
    let records: serde_json::Value =
        serde_json::from_str(&output.deferred_defaults()).map_err(|error| error.to_string())?;
    // Then the typed roots, cut and authored location survive serialization/getter copies.
    assert_eq!(output.code(), source);
    assert_eq!(
        records,
        serde_json::json!([{
            "extraction": "/page.tsx", "request": request, "exports": ["default"],
            "line": 2, "column": 8, "demand": "render-only", "attributes": false,
            "cutImporter": if forwarded { "/forward.js" } else { "/page.tsx" },
            "cutRequest": "./doc.mdx", "cutExport": "default",
        }])
    );
    Ok(())
}

#[test]
#[serial]
fn empty_deferred_output_remains_explicit() -> Result<(), String> {
    let _state = TestStateGuard::new();
    let output = code_extract_internal(
        "/empty.ts",
        "export const value=1;",
        "@devup-ui/react",
        "df".to_string(),
        false,
        false,
        false,
        HashMap::new(),
        None,
    )?;
    assert_eq!(output.deferred_defaults(), "[]");
    Ok(())
}
