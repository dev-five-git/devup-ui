use css::{Site, sparse_site::SourceFile};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, ResolvedModule, extract_with_modules};
use serial_test::serial;

#[test]
#[serial]
fn whole_assignment_site_survives_when_imported_flag_changes() {
    // Given: identical authored source with two runtime-valued branches.
    let file = "/src/assignment.tsx";
    let source = "import {Box} from '@devup-ui/react';import {FLAG} from './flag';export const View=({left,right})=><Box color={FLAG ? left : right}/>;";
    let site = Site {
        file: SourceFile::D9(0),
        at: source
            .find("FLAG ?")
            .unwrap_or_else(|| panic!("fixture assignment")),
        role: 0,
    };
    let mut actual = Vec::new();
    for flag in [true, false] {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        css::file_map::reset_canonical_map();
        css::file_map::seed_file_numbers(&[file.to_string()]);
        css::set_prefix(None);
        let resolver = move |_: &str, _: &str| {
            Some(ResolvedModule {
                path: "/src/flag.ts".to_string(),
                code: format!("export const FLAG={flag};"),
            })
        };
        // When: the import resolves to each boolean without changing the source.
        let output = extract_with_modules(file, source, ExtractOption::default(), false, &resolver)
            .unwrap_or_else(|error| panic!("{error}"));
        let variables = output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Dynamic(style) => Some(style.variable_name()),
                _ => None,
            })
            .collect::<Vec<_>>();
        // Then: the selected runtime value still owns the whole assignment's one site.
        actual.push((flag, variables));
    }
    assert_eq!(
        actual,
        vec![
            (true, vec![site.variable_name("")]),
            (false, vec![site.variable_name("")]),
        ]
    );
}

#[test]
#[serial]
fn whole_site_survives_when_logical_and_nested_branches_fold() {
    // Given: selected runtime values behind all three logical operators and nested folds.
    let file = "/src/assignment.tsx";
    let fixtures = [
        ("FLAG && left", "true", "left"),
        ("FLAG || right", "false", "right"),
        ("FLAG ?? right", "null", "right"),
        ("FLAG ? (FLAG ? left : right) : right", "true", "left"),
        ("FLAG ? left : (FLAG ? left : right)", "false", "right"),
        ("FLAG && (FLAG ? left : right)", "true", "left"),
        ("FLAG || (FLAG ?? right)", "null", "right"),
        ("FLAG ?? (false || right)", "null", "right"),
    ];
    for (expression, flag, selected) in fixtures {
        let normalized = format!(
            "import {{Box}} from '@devup-ui/react';\nimport {{FLAG}} from './flag';\nexport const View=({{left,right}})=><Box color={{{expression}}}/>;"
        );
        let site = Site {
            file: SourceFile::D9(0),
            at: normalized
                .find(expression)
                .unwrap_or_else(|| panic!("fixture expression")),
            role: 0,
        };
        for source in [
            normalized.clone(),
            normalized.replace('\n', "\r\n"),
            format!("\u{feff}{normalized}"),
            format!("\u{feff}{}", normalized.replace('\n', "\r\n")),
        ] {
            css::class_map::reset_class_map();
            css::file_map::reset_file_map();
            css::file_map::reset_canonical_map();
            css::file_map::seed_file_numbers(&[file.to_string()]);
            css::set_prefix(None);
            let resolver = move |_: &str, _: &str| {
                Some(ResolvedModule {
                    path: "/src/flag.ts".to_string(),
                    code: format!("export const FLAG={flag};"),
                })
            };
            // When: extraction folds the imported controller with unchanged selection semantics.
            let output =
                extract_with_modules(file, &source, ExtractOption::default(), false, &resolver)
                    .unwrap_or_else(|error| panic!("{expression}: {error}"));
            let values = output
                .styles
                .iter()
                .filter_map(|style| match style {
                    ExtractStyleValue::Dynamic(style) => {
                        Some((style.variable_name(), style.identifier().to_string()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            // Then: the original whole site names the actual selected branch, not its location.
            assert_eq!(
                values,
                vec![(site.variable_name(""), selected.to_string())],
                "{expression}"
            );
        }
    }
}

#[test]
#[serial]
fn whole_site_keeps_responsive_roles_when_selected_array_changes() {
    // Given: one authored value can select either responsive runtime array.
    let file = "/src/assignment.tsx";
    let source = "import {Box} from '@devup-ui/react';import {FLAG} from './flag';export const View=({left,right})=><Box color={FLAG ? [left,right] : [right,left]}/>;";
    let at = source
        .find("FLAG ?")
        .unwrap_or_else(|| panic!("fixture expression"));
    for flag in [true, false] {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        css::file_map::reset_canonical_map();
        css::file_map::seed_file_numbers(&[file.to_string()]);
        css::set_prefix(None);
        let resolver = move |_: &str, _: &str| {
            Some(ResolvedModule {
                path: "/src/flag.ts".to_string(),
                code: format!("export const FLAG={flag};"),
            })
        };
        // When: the inliner chooses the array before responsive extraction.
        let output = extract_with_modules(file, source, ExtractOption::default(), false, &resolver)
            .unwrap_or_else(|error| panic!("{error}"));
        let mut actual = output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Dynamic(style) => {
                    Some((style.variable_name(), style.identifier().to_string()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        actual.sort();
        let selected = if flag {
            ["left", "right"]
        } else {
            ["right", "left"]
        };
        let expected = selected
            .into_iter()
            .enumerate()
            .map(|(role, value)| {
                let site = Site {
                    file: SourceFile::D9(0),
                    at,
                    role,
                };
                (site.variable_name(""), value.to_string())
            })
            .collect::<Vec<_>>();
        // Then: roles stay source-ordered under the whole owner while values may differ.
        assert_eq!(actual, expected);
    }
}
