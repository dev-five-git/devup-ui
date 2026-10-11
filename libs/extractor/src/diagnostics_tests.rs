use crate::{ExtractOption, extract_without_source_map};
use serial_test::serial;

#[test]
#[serial]
fn fatal_parse_reports_original_file_and_first_parser_error() {
    let source = "import { Box } from '@devup-ui/react';\nexport const view = <Box bg={ />;";

    let error = extract_without_source_map("/src/broken.tsx", source, ExtractOption::default())
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();

    assert!(error.starts_with("/src/broken.tsx:2:31:"), "{error}");
    assert!(error.contains("Unterminated regular expression"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
}

#[test]
#[serial]
fn unsupported_extension_reports_the_input_file_and_repair() {
    let source = "import { Box } from '@devup-ui/react';";

    let error = extract_without_source_map("/src/broken.unknown", source, ExtractOption::default())
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();

    assert!(error.starts_with("/src/broken.unknown:1:1:"), "{error}");
    assert!(error.contains("Unknown file extension"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
}

#[rstest::rstest]
#[case("\n")]
#[case("\r")]
#[case("\r\n")]
#[case("\u{2028}")]
#[case("\u{2029}")]
fn locations_follow_javascript_line_terminators(#[case] separator: &str) {
    let source = format!("first{separator}한😀x");

    let location = crate::locate("/src/unicode.ts", &source, source.len());

    assert_eq!(location, "/src/unicode.ts:2:4");
}

#[rstest::rstest]
#[case("return 1;")]
#[case("const invalid = /[z-a]/;")]
#[case("const repeated = 1; const repeated = 2;")]
#[case("const invalid = 01;")]
#[serial]
fn recoverable_and_semantic_errors_do_not_publish_css(#[case] invalid: &str) {
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{invalid}\nexport const card = css({{ color: 'red' }});"
    );

    let error = extract_without_source_map("/src/syntax.tsx", &source, ExtractOption::default())
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();

    assert!(
        error.starts_with("/src/syntax.tsx:2:"),
        "{invalid}: {error}"
    );
    assert!(error.contains("Cannot parse source:"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
}

#[rstest::rstest]
#[case("<Box wrong={} />")]
#[case("<Box className={} />")]
#[case("<Box styleOrder={} />")]
#[case("<Box styleVars={} />")]
#[case("<div css={} />")]
#[serial]
fn empty_jsx_attribute_expressions_are_syntax_errors_not_absent_values(#[case] element: &str) {
    let source = format!("import {{ Box }} from '@devup-ui/react';\nexport const App = {element};");

    let error = extract_without_source_map("/src/empty.tsx", &source, ExtractOption::default())
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();

    assert!(error.starts_with("/src/empty.tsx:2:"), "{error}");
    assert!(error.contains("Cannot parse source:"), "{error}");
    assert!(error.contains("non-empty 'expression'"), "{error}");
}
