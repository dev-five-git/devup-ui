use extractor::extract_style::{
    extract_style_value::ExtractStyleValue, style_property::StyleProperty,
};
use extractor::{ExtractOption, ResolvedModule, extract, extract_with_modules};
use rstest::rstest;
use serial_test::serial;

fn reset_names(file: &str) {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_map::seed_file_numbers(&[file.to_string()]);
    css::debug::set_debug(false);
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
}

fn class_names(output: &extractor::ExtractOutput, file: &str) -> Vec<String> {
    let mut names: Vec<_> = output
        .styles
        .iter()
        .map(|style| {
            match style
                .extract(Some(file))
                .unwrap_or_else(|| panic!("missing class identity: {style:?}"))
            {
                StyleProperty::ClassName(name)
                | StyleProperty::Variable {
                    class_name: name, ..
                } => name,
            }
        })
        .collect();
    names.sort();
    names
}

fn assert_imported_atoms_leave_padding_first(output: &extractor::ExtractOutput, file: &str) {
    let mut padding = 0;
    let mut imported = 0;
    for style in &output.styles {
        let property = match style {
            ExtractStyleValue::Static(style) => style.property(),
            ExtractStyleValue::Dynamic(style) => style.property(),
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::Keyframes(_) => panic!("unexpected style record: {style:?}"),
        };
        let identity = style
            .extract(Some(file))
            .unwrap_or_else(|| panic!("missing class identity: {style:?}"));
        let class_name = match identity {
            StyleProperty::ClassName(name)
            | StyleProperty::Variable {
                class_name: name, ..
            } => name,
        };
        if property == "padding" {
            assert_eq!(class_name, "a-a", "{style:?}");
            assert!(matches!(style, ExtractStyleValue::Static(style) if style.value() == "16px"));
            padding += 1;
        } else {
            assert!(
                class_name.starts_with("a-RL") || class_name.starts_with("a-RH"),
                "imported atom must be content-named: {class_name} {style:?}"
            );
            imported += 1;
        }
    }
    assert_eq!(padding, 1);
    assert!(imported > 0, "fixture must emit imported atoms");
    css::class_map::with_class_map(|map| {
        assert_eq!(
            map.values()
                .map(std::collections::HashMap::len)
                .sum::<usize>(),
            1
        );
    });
}

#[rstest]
#[case(
    "/src/local.ts",
    "import {css} from '@devup-ui/react';const a='red';const b=a;export const c=css({color:b,padding:'16px'});"
)]
#[case(
    "/src/local.ts",
    "import {css} from '@devup-ui/react';function a(){return 'red'}const b=()=>a();export const c=css({color:b(),padding:'16px'});"
)]
#[case(
    "/src/local.css.ts",
    "import {style} from '@devup-ui/react';export const c=style({color:'red',padding:'16px'});"
)]
#[case(
    "/src/local.ts",
    "import {css} from '@devup-ui/react';const a='$primary';const b=a;export const c=css({color:b,padding:'16px'});"
)]
#[serial]
fn own_names_when_all_initializers_are_local(#[case] file: &str, #[case] source: &str) {
    // Given: no style data enters from another module.
    reset_names(file);
    // When: the public extractor evaluates the local initializer chain.
    let output =
        extract(file, source, ExtractOption::default()).unwrap_or_else(|error| panic!("{error}"));
    // Then: only short classes are emitted.
    assert_eq!(class_names(&output, file), vec!["a-a", "a-b"]);
}

#[rstest]
#[case("import {value} from './data';", "value")]
#[case("import value from './data';", "value")]
#[case("import * as data from './data';", "data.value")]
#[case("const data=require('./data');", "data.value")]
#[case("const data=import('./data');", "data.value")]
#[serial]
fn imported_names_when_no_resolver_certifies_them(
    #[case] declaration: &str,
    #[case] expression: &str,
) {
    // Given: an import remains unknown without a resolver.
    reset_names("/src/local.tsx");
    let source = format!(
        "import {{Box}} from '@devup-ui/react';{declaration}export const c=<Box color={{{expression}}} p={{4}}/>;"
    );
    // When: the public extractor emits a dynamic style.
    let output = extract("/src/local.tsx", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    // Then: the imported atom is content-named and padding owns the first slot.
    assert_imported_atoms_leave_padding_first(&output, "/src/local.tsx");
}

#[test]
#[serial]
fn equal_imported_and_local_values_keep_disjoint_identities() {
    // Given: imported red is equal to a later local red.
    reset_names("/src/shared.ts");
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/src/data.ts".to_string(),
            code: "export const value='red';".to_string(),
        })
    };
    // When: both styles are extracted through the public API.
    let output = extract_with_modules("/src/shared.ts", concat!("import {css} from '@devup-ui/react';import {", "value", "} from './data';export const a=css({color:value});export const b=css({color:'red',padding:'16px'});"), ExtractOption::default(), false, &resolver).unwrap_or_else(|error| panic!("{error}"));
    // Then: equal CSS content cannot give the import a local counter identity.
    let names = class_names(&output, "/src/shared.ts");
    assert_eq!(
        names
            .iter()
            .filter(|name| name.starts_with("a-RL") || name.starts_with("a-RH"))
            .count(),
        1
    );
    assert_eq!(
        names
            .iter()
            .filter(|name| !name.starts_with("a-RL") && !name.starts_with("a-RH"))
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["a-a", "a-b"]
    );
    assert_eq!(output.styles.len(), 3);
}

#[test]
#[serial]
fn marked_errors_keep_original_location_and_expression() {
    // Given: a source import is marked before an invalid css value is reported.
    reset_names("/src/error.ts");
    let source = concat!(
        "import {css} from '@devup-ui/react';\nimport {",
        "value",
        "} from './data';\nexport const a=css({color:value});"
    );
    // When: an unresolved data import cannot supply a static API value.
    let Err(error) = extract("/src/error.ts", source, ExtractOption::default()) else {
        panic!("expected unresolved static API value")
    };
    let error = error.to_string();
    // Then: the mark never leaks into the reported byte offset or code.
    assert!(error.contains("/src/error.ts:3:16:"), "{error}");
    assert!(error.contains("`value`"), "{error}");
    assert!(error.contains("build time"), "{error}");
}

#[rstest]
#[case("{[value]:'red'}", "'color'")]
#[case("{...(value ? {color:'red'} : {})}", "true")]
#[case("{color:[...value,'green']}", "['red','blue']")]
#[serial]
fn structural_imports_leave_later_local_slots_untouched(#[case] rules: &str, #[case] value: &str) {
    // Given: external data controls a key, presence, or responsive shape.
    reset_names("/src/shared.ts");
    let module = format!("export const value={value};");
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/src/data.ts".to_string(),
            code: module.clone(),
        })
    };
    let source = format!(
        "import {{css}} from '@devup-ui/react';import {{value}} from './data';export const a=css({rules});export const b=css({{padding:'16px'}});"
    );
    // When: the public extractor resolves structural style data.
    let output = extract_with_modules(
        "/src/shared.ts",
        &source,
        ExtractOption::default(),
        true,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then: later local padding still owns the first slot.
    assert_imported_atoms_leave_padding_first(&output, "/src/shared.ts");
}
