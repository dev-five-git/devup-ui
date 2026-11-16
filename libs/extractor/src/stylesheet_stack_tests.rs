use crate::{ExtractOption, ExtractStyleValue, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

/// A stylesheet is instrumented before Boa runs it, so each authored call nests
/// a few more expressions; Boa parses and compiles them recursively. This runs on
/// the 2 MiB stack every test thread gets, which unoptimized Boa overflowed.
#[rstest]
#[case::nested_chain("g(f(1)).d(3).toString()", "4px")]
#[case::deeper_argument("g(f(f(f(1)))).d(3).toString()", "4px")]
#[case::chain_in_chain("g(g(f(1)).d(2).toString()).d(3).toString()", "6px")]
#[case::three_chains(
    "g(g(g(f(f(1))).d(2).toString()).d(2).toString()).d(3).toString()",
    "8px"
)]
#[serial]
fn nested_chained_call_fits_the_test_thread_stack(#[case] width: &str, #[case] expected: &str) {
    // Given: a vanilla-extract stylesheet reading a call nested in a chained call.
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{ style }} from '@vanilla-extract/css';\nconst f = (x) => x;\nconst g = (x) => ({{ d: (y) => ({{ toString: () => `${{Number.parseInt(`${{x}}`, 10) + y}}px` }}) }});\nexport const a = style({{ width: {width} }});"
    );
    let option = ExtractOption {
        import_aliases: HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };

    // When
    let output = extract("/src/a.css.ts", &code, option).map_err(|error| error.to_string());

    // Then: it is evaluated to the width it computes.
    let widths: Vec<String> = output
        .iter()
        .flat_map(|output| &output.styles)
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "width" => {
                Some(style.value.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(widths, [expected], "{output:?}");
}
