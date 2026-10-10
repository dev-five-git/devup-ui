use super::*;

fn compiled(source: &str, mode: (bool, bool, bool)) -> (String, String, BTreeSet<String>) {
    reset(mode.0, mode.1);
    let mut option = ExtractOption {
        single_css: mode.2,
        ..ExtractOption::default()
    };
    option.import_aliases.insert(
        "@emotion/styled".to_string(),
        extractor::ImportAlias::DefaultToNamed("styled".to_string()),
    );
    let output = extract(FILE, source, option)
        .unwrap_or_else(|error| panic!("sparse site extraction failed: {error}"));
    let variables = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => Some(style.variable_name()),
            _ => None,
        })
        .collect();
    let mut sheet = StyleSheet::default();
    sheet
        .update_styles(&output.styles, FILE, mode.2)
        .unwrap_or_else(|error| panic!("{error}"));
    (
        output.code,
        sheet.create_css((!mode.2).then_some(FILE), false),
        variables,
    )
}

#[rstest]
#[case(false, false, false)]
#[case(true, false, false)]
#[case(false, true, false)]
#[case(true, true, false)]
#[case(false, false, true)]
#[case(true, false, true)]
#[case(false, true, true)]
#[case(true, true, true)]
#[serial]
fn full_outputs_match_when_line_endings_and_leading_bom_change(
    #[case] debug: bool,
    #[case] atom: bool,
    #[case] single: bool,
) {
    // Given: alias edits and multiple composite values sharing a template span.
    let source = "import styled from '@emotion/styled';\nexport const View=styled.div`\n padding: ${p=>p.pad}px;\n margin: ${p=>p.gap}em;\n`;\n";
    let expected = compiled(source, (debug, atom, single));
    // When: the same source is received with CRLF, with and without a BOM.
    for input in [
        source.replace('\n', "\r\n"),
        format!("\u{feff}{source}"),
        format!("\u{feff}{}", source.replace('\n', "\r\n")),
    ] {
        let actual = compiled(&input, (debug, atom, single));
        // Then: full JS, CSS (including resets), and identities agree byte for byte.
        assert_eq!(actual, expected);
    }
}

#[test]
#[serial]
fn golden_site_uses_d9_and_received_expression_offset_in_base37() {
    // Given: D9 source 27 and an expression starting at received-source byte 52.
    reset(false, false);
    css::file_map::set_original_ids(BTreeMap::from([(FILE.to_string(), 27)]));
    let source = "import {Box} from '@devup-ui/react';const V=<Box p={v}/>;";
    // When: the real extractor chooses the sparse identity.
    let output = extract(FILE, source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("golden extraction failed: {error}"));
    let names: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => Some(style.variable_name()),
            _ => None,
        })
        .collect();
    // Then: both coordinates use base37, not decimal or extraction counters.
    assert_eq!(names, ["---test-Saa-az"]);
}

#[test]
#[serial]
fn stylex_site_uses_value_expression_offset_when_property_key_is_earlier() {
    // Given: a StyleX property with distinct key and value-expression positions.
    reset(false, false);
    let source = "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({dynamic:(amount)=>({width: /* value */ amount})});export const View=({amount})=><div {...stylex.props(styles.dynamic(amount))}/>;";
    let at = source
        .find("amount})")
        .unwrap_or_else(|| panic!("fixture must contain the value expression"));
    let key_at = source
        .find("width:")
        .unwrap_or_else(|| panic!("fixture must contain the property key"));
    assert!(key_at < at);
    let expected = css::Site {
        file: css::sparse_site::SourceFile::D9(0),
        at,
        role: 0,
    }
    .variable_name("test-");
    let key_name = css::Site {
        file: css::sparse_site::SourceFile::D9(0),
        at: key_at,
        role: 0,
    }
    .variable_name("test-");
    // When: the extractor assigns a sparse identity to the dynamic StyleX value.
    let output = extract(FILE, source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("StyleX position extraction failed: {error}"));
    let names: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => Some(style.variable_name()),
            _ => None,
        })
        .collect();
    // Then: the source-derived golden uses the value offset, never the property key.
    assert_eq!(names, [expected]);
    assert_ne!(names, [key_name]);
}

#[test]
#[serial]
fn unnumbered_names_use_source_alone_when_filenames_and_arrival_order_change() {
    // Given: identical, distinct and BOM/CRLF-normalized sources without D9 numbers.
    let source = "import {Box} from '@devup-ui/react';export const View=({pad})=><Box p={pad}/>;";
    let different = format!("{source}// distinct received source");
    let normalized = format!("\u{feff}{}", source.replace(';', ";\r\n"));
    let lf = source.replace(';', ";\n");
    let names = |reverse| {
        reset(false, false);
        css::file_map::set_original_ids(BTreeMap::new());
        let mut inputs = [
            ("/unseeded/first.tsx", source),
            ("/different/root/second.tsx", source),
            ("/unseeded/third.tsx", different.as_str()),
            ("/unseeded/lf.tsx", lf.as_str()),
            ("/unseeded/bom-crlf.tsx", normalized.as_str()),
        ];
        if reverse {
            inputs.reverse();
        }
        inputs
            .into_iter()
            .map(|(filename, input)| {
                let output = extract(filename, input, ExtractOption::default())
                    .unwrap_or_else(|error| panic!("unnumbered extraction failed: {error}"));
                let variables = output
                    .styles
                    .into_iter()
                    .filter_map(|style| match style {
                        ExtractStyleValue::Dynamic(style) => Some(style.variable_name()),
                        _ => None,
                    })
                    .collect::<BTreeSet<_>>();
                (filename, variables)
            })
            .collect::<BTreeMap<_, _>>()
    };
    // When: every file is extracted in both arrival orders.
    let first = names(false);
    let later = names(true);
    // Then: full normalized source, never filename or arrival order, determines each name.
    assert_eq!(first, later);
    let identical = &first["/unseeded/first.tsx"];
    assert_eq!(identical.len(), 1);
    assert_eq!(identical, &first["/different/root/second.tsx"]);
    assert_ne!(identical, &first["/unseeded/third.tsx"]);
    assert_eq!(first["/unseeded/lf.tsx"], first["/unseeded/bom-crlf.tsx"]);
    assert!(identical.iter().all(|name| name.starts_with("---test-SU")));
}

#[test]
#[serial]
fn later_template_role_stays_named_when_an_earlier_import_becomes_dynamic() {
    // Given: two source interpolations; an import is static in only one environment.
    let source = "import styled from '@emotion/styled';import {FIRST} from 'conditional';export const View=styled.div`padding:${FIRST}px;margin:${p=>p.gap}em;`;";
    let environment = |known: bool| {
        reset(false, false);
        let mut option = ExtractOption::default();
        option.import_aliases.insert(
            "@emotion/styled".to_string(),
            extractor::ImportAlias::DefaultToNamed("styled".to_string()),
        );
        let resolver = move |_: &str, _: &str| {
            known.then(|| ResolvedModule {
                path: "/pkg/conditional.ts".to_string(),
                code: "export const FIRST=1;".to_string(),
            })
        };
        let output = extract_with_modules(FILE, source, option, false, &resolver)
            .unwrap_or_else(|error| panic!("template role extraction failed: {error}"));
        let dynamic: Vec<_> = output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Dynamic(style) => Some(style),
                _ => None,
            })
            .collect();
        let later: Vec<_> = dynamic
            .iter()
            .filter(|style| style.property() == "margin")
            .map(|style| style.variable_name())
            .collect();
        (dynamic.len(), later)
    };
    // When: the earlier interpolation changes its dynamic eligibility.
    let known = environment(true);
    let runtime = environment(false);
    // Then: a source-order role cannot shift with the count of extracted dynamic values.
    assert_eq!(known.0, 1);
    assert_eq!(runtime.0, 2);
    assert_eq!(known.1.len(), 1);
    assert_eq!(known.1, runtime.1);
}

#[test]
#[serial]
fn generated_namespace_stays_disjoint_when_theme_tokens_resemble_site_names() {
    // Given: legitimate theme keys occupying the old and plausible generated grammars.
    let theme: sheet::theme::Theme = serde_json::from_str(r##"{
        "colors":{"default":{"test-S0-121":"#f00","test-Sa-a":"#00f","test-Saa-ad-b":"#f0f","test.SUfoo.a":"#0f0","_token":"#000","1token":"#fff"}}
    }"##).unwrap_or_else(|error| panic!("legitimate theme keys must remain accepted: {error}"));
    reset(false, false);
    let mut sheet = StyleSheet::default();
    sheet.set_theme(theme);
    let theme_css = sheet.create_css(None, false);
    let source = "import {Box} from '@devup-ui/react';export const View=({pad})=><Box p={pad} color=\"$test-Sa-a\"/>;";
    // When: the real extractor names an owner variable alongside theme variables.
    let output = extract(FILE, source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("theme extraction failed: {error}"));
    let variables: BTreeSet<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => Some(style.variable_name()),
            _ => None,
        })
        .collect();
    // Then: the theme remains intact; generated and legacy namespaces cannot equal it.
    assert!(theme_css.contains("--test-Sa-a:"));
    assert_eq!(variables.len(), 1);
    for variable in variables {
        assert!(variable.starts_with("---test-S"));
        assert!(!theme_css.contains(&format!("{variable}:")));
        for mode in [(false, false), (true, false), (false, true), (true, true)] {
            css::debug::set_debug(mode.0);
            css::atom_hoist::set_atom_hoist(mode.1.then_some(1));
            assert_ne!(variable, css::sheet_to_variable_name("padding", 0, None));
        }
    }
    css::debug::set_debug(false);
    css::atom_hoist::set_atom_hoist(None);
    sheet.set_theme(sheet::theme::Theme::default());
}
