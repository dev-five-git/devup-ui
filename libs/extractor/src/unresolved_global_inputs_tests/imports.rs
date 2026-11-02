use super::rejects_at;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("[{url: unknownUrl}]", "unknownUrl")]
#[case("[{url: tick()}]", "tick()")]
#[case("[{url: 'base.css', query: unknownQuery}]", "unknownQuery")]
#[case("[{url: 'base.css', query: tick()}]", "tick()")]
#[case("[unknownEntry]", "unknownEntry")]
#[case("[tick()]", "tick()")]
#[case("unknownImports", "unknownImports")]
#[case("tick()", "tick()")]
#[case("['first.css', unknownEntry, 'last.css']", "unknownEntry")]
#[case("[...unknownEntries]", "unknownEntries")]
#[case("[...tick()]", "tick()")]
#[case("[{...unknownEntry, url: 'base.css'}]", "unknownEntry")]
#[case("[{url: 'base.css', ...tick()}]", "tick()")]
#[case("[{url: 'base.css', query: void tick()}]", "void tick()")]
#[serial]
fn rejects_unresolved_imports_when_present(
    #[case] value: &str,
    #[case] marker: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nglobalCss({{ imports: {value} }});"
    );
    rejects_at(&source, marker, single_css);
}

#[rstest]
#[case("unknownKey")]
#[case("tick()")]
#[serial]
fn rejects_unresolved_import_keys_at_the_key_expression(
    #[case] key: &str,
    #[values(false, true)] single_css: bool,
) {
    let statement = format!("globalCss({{imports:[{{[{key}]:'base.css'}}]}});");
    let column = crate::dead_properties_test_utils::position(&statement, key) + 1;
    let source = format!("import {{globalCss}} from '@devup-ui/react';\n{statement}");
    let error =
        crate::dead_properties_test_utils::failure(super::extract_fixture(&source, single_css));
    assert_eq!(
        error
            .split_once(": ")
            .unwrap_or_else(|| panic!("unlocated error: {error}"))
            .0,
        format!("structural.tsx:2:{column}")
    );
    assert!(error.contains(&format!("cannot use `[{key}]` at build time:")));
}

#[rstest]
#[case::query("imports: [{url: 'base.css', query: undefined}]")]
#[case::descriptor("fontFaces: [{fontFamily: 'Face', src: 'face.woff2', fontStyle: undefined}]")]
#[case::params("body: {_nthChild: {params: [undefined], color: 'red'}}")]
#[serial]
fn rejects_shadowed_undefined_when_structural(
    #[case] properties: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nexport function render(undefined) {{\n  globalCss({{ {properties} }});\n}}"
    );
    // The parameter's spelling is deliberately identical to the unbound global.
    let use_offset = source
        .rfind("undefined")
        .unwrap_or_else(|| panic!("missing fixture reference"));
    let error =
        crate::dead_properties_test_utils::failure(super::extract_fixture(&source, single_css));
    let line = &source[..use_offset];
    let column = line
        .rsplit_once('\n')
        .map_or(line, |(_, tail)| tail)
        .chars()
        .count()
        + 1;
    assert_eq!(
        error
            .split_once(": ")
            .unwrap_or_else(|| panic!("unlocated error: {error}"))
            .0,
        format!("structural.tsx:3:{column}")
    );
    assert!(error.contains("cannot use `undefined` at build time:"));
}

#[rstest]
#[case("imports: [{url: 'base.css', query: unknownQuery}]", "unknownQuery")]
#[case("fontFaces: [{fontFamily: 'Face', src: tick()}]", "tick()")]
#[case("fontFaces: [`font-family: Face; src: ${unknownSrc};`]", "unknownSrc")]
#[case(
    "body: {_nthChild: {params: ['2', unknownParam], color: 'red'}}",
    "unknownParam"
)]
#[serial]
fn emotion_global_reports_the_marker_not_styles_container(
    #[case] properties: &str,
    #[case] marker: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ Global }} from '@emotion/react';\nexport const view = <Global styles={{{{\n  {properties}\n}}}} />;"
    );
    rejects_at(&source, marker, single_css);
}

#[rstest]
#[case("[{url: 'base.css'}]", "\"base.css\"")]
#[case(
    "[{url: `base${'.css'}`, query: `scr${'een'}`}]",
    "\"base.css\" screen"
)]
#[case("[{url: 'base.css', query: undefined}]", "\"base.css\"")]
#[case("[{url: 'base.css', query: null}]", "\"base.css\"")]
#[case("[{url: 'base.css', query: false}]", "\"base.css\" false")]
#[case("['base.css']", "base.css")]
#[serial]
fn readable_imports_preserve_text_and_genuine_optional_absence(
    #[case] value: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nglobalCss({{ imports: {value} }});"
    );
    let output =
        super::extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let imports: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::Import(import) => import.url.as_str(),
            other => panic!("expected import, got {other:?}"),
        })
        .collect();
    assert_eq!(imports, vec![expected]);
}

#[rstest]
#[serial]
fn pure_invalid_imports_remain_skipped(
    #[values("[1, 2, {query: 'print'}]", "{}", "null", "17")] value: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nglobalCss({{ imports: {value} }});"
    );
    let output =
        super::extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(output.styles.len(), 0);
}
