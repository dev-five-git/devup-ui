use std::collections::BTreeMap;

use css::{Site, sparse_site::SourceFile};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, ExtractOutput, ResolvedModule, extract, extract_with_modules};
use rstest::rstest;
use serial_test::serial;

const FILE: &str = "/src/span.tsx";

fn reset_names() {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_map::seed_file_numbers(&[FILE.to_string()]);
    css::debug::set_debug(false);
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
}

fn dynamic_names(output: &ExtractOutput) -> BTreeMap<String, String> {
    output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => {
                Some((style.property().to_string(), style.variable_name()))
            }
            _ => None,
        })
        .collect()
}

fn authored_name(source: &str, expression: &str, role: usize) -> String {
    let site = Site {
        file: SourceFile::from_source(FILE, source),
        at: source
            .find(expression)
            .unwrap_or_else(|| panic!("fixture has authored expression")),
        role,
    };
    site.variable_name(css::get_prefix().as_deref().unwrap_or_default())
}

#[rstest]
#[case("theme.colors[key]", "({ a: 'red', ...theme.more })['b']")]
#[case("colors[key]", "PRIMARY['length']")]
#[case("[...items][index]", "[...other][index]")]
#[case("[...items][9]", "[...other][9]")]
#[case("({ a: 'red', ...theme.more })['a']", "theme.colors[key]")]
#[serial]
fn variables_use_authored_offsets_when_members_are_rebuilt(
    #[case] first: &str,
    #[case] second: &str,
) {
    // Given: two runtime members whose ASTs must be reconstructed.
    reset_names();
    let source = format!(
        "import {{ Box }} from '@devup-ui/react';\nexport const View = (theme, colors, PRIMARY, items, other, key, index) => <Box w={{{first}}} h={{{second}}} />;"
    );
    // When: extraction constructs each generated computed-member expression.
    let output =
        extract(FILE, &source, ExtractOption::default()).unwrap_or_else(|error| panic!("{error}"));
    // Then: both assignments retain their own authored byte offset, not SPAN zero.
    assert_eq!(
        dynamic_names(&output),
        BTreeMap::from([
            ("width".to_string(), authored_name(&source, first, 0)),
            ("height".to_string(), authored_name(&source, second, 0)),
        ])
    );
}

#[rstest]
#[case(false, false)]
#[case(true, false)]
#[case(false, true)]
#[case(true, true)]
#[serial]
fn imported_member_offsets_survive_own_edits_when_line_endings_change(
    #[case] crlf: bool,
    #[case] bom: bool,
) {
    // Given: an alias rewrite before imported-literal and unresolved-member reads.
    reset_names();
    let source = "import styled from '@emotion/styled';\nimport { Box } from '@devup-ui/react';\nimport { PRIMARY, colors } from './tokens';\n// 한글 precedes byte offsets\nexport const View = (key) => <Box w={PRIMARY['length']} h={colors[key]} />;";
    let received = format!(
        "{}{}",
        if bom { "\u{feff}" } else { "" },
        if crlf {
            source.replace('\n', "\r\n")
        } else {
            source.to_string()
        }
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./tokens").then(|| ResolvedModule {
            path: "/src/tokens.ts".to_string(),
            code: "export const PRIMARY = 'red'; export const colors = { primary: 'blue' };"
                .to_string(),
        })
    };
    // When: inlining and alias rewriting feed native extraction.
    let output = extract_with_modules(FILE, &received, ExtractOption::default(), false, &resolver)
        .unwrap_or_else(|error| panic!("{error}"));
    // Then: names use normalized received-source offsets, not rewritten or generated ones.
    assert_eq!(
        dynamic_names(&output),
        BTreeMap::from([
            (
                "width".to_string(),
                authored_name(source, "PRIMARY['length']", 0),
            ),
            (
                "height".to_string(),
                authored_name(source, "colors[key]", 0),
            ),
        ])
    );
}

#[test]
#[serial]
fn source_order_roles_stay_injective_when_template_values_share_one_span() {
    // Given: composite assignments share the authored template's expression span.
    reset_names();
    let source = "import { styled } from '@devup-ui/react';\nexport const Card = styled.div`width: ${left}px; height: ${right}px;`;";
    // When: both composite values are extracted from that template.
    let output =
        extract(FILE, source, ExtractOption::default()).unwrap_or_else(|error| panic!("{error}"));
    // Then: interpolation source order supplies distinct subroles at the same offset.
    assert_eq!(
        dynamic_names(&output),
        BTreeMap::from([
            ("width".to_string(), authored_name(source, "`width:", 1)),
            ("height".to_string(), authored_name(source, "`width:", 2)),
        ])
    );
}
