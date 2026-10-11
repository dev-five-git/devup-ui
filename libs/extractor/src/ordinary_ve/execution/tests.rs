use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use super::{Captured, Executed, SelectedModule, Stylesheet, execute};
use crate::{ExtractOption, ImportAlias, ModuleResolver};

mod catalog;
mod failures;
mod imports;

fn option() -> ExtractOption {
    ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from_iter([(
            "@vanilla-extract/css".into(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

fn run(stylesheet: Stylesheet<'_>, resolver: Option<&ModuleResolver>) -> Result<Executed, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        stylesheet.code,
        SourceType::from_path(stylesheet.filename).unwrap_or_default(),
    )
    .parse();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let plan = crate::ordinary_ve::selection::select(&parsed.program, &semantic);
    execute(
        SelectedModule {
            stylesheet,
            selection: &plan,
        },
        &option(),
        resolver,
    )
}

fn written<'a>(filename: &'a str, code: &'a str) -> Stylesheet<'a> {
    Stylesheet {
        filename,
        code,
        source: code,
        edits: &[],
    }
}

fn binding<'a>(result: &'a Executed, name: &str) -> &'a Captured {
    result
        .captures
        .iter()
        .find(|capture| {
            capture
                .binding
                .as_ref()
                .is_some_and(|binding| binding.name == name)
        })
        .unwrap_or_else(|| panic!("missing captured binding {name}"))
}

#[rstest]
#[case("ts", "")]
#[case("tsx", "const view=<div/>;")]
#[case("js", "")]
#[case("jsx", "const view=<div/>;")]
#[case("mjs", "")]
#[serial]
fn evaluates_native_slices_when_host_siblings_throw_or_render_jsx(
    #[case] extension: &str,
    #[case] jsx: &str,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = format!(
        "import {{createTheme,style}} from '@vanilla-extract/css';import React from 'never-resolve';const tokens={{space:'8px'}},browser=window.document;export const [theme,vars]=createTheme(tokens);const box=style({{margin:vars.space}});{jsx}throw new Error('runtime only');"
    );
    let filename = format!("/sliced.{extension}");
    // When
    let result = run(written(&filename, &code), None)?;
    // Then
    assert_eq!(result.collected.styles.len(), 1);
    assert!(
        binding(&result, "vars")
            .expression
            .contains("var(--space-0-1)")
    );
    assert_eq!(
        result.collected.global_styles[0].1,
        "{\"--space-0-1\":\"8px\"}"
    );
    assert!(result.imports.dependencies.is_empty());
    Ok(())
}

#[test]
#[serial]
fn captures_once_when_a_hoisted_native_helper_is_called_twice() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = "import {createVar,createContainer,style} from '@vanilla-extract/css';const before=createVar();const first=make(),host=window.name;function make(){const value=createVar();const container=createContainer();return {value,container,box:style({margin:value})}}const second=make();const after=createVar();";
    // When
    let result = run(written("/counter.ts", code), None)?;
    // Then
    assert_eq!(result.collected.styles.len(), 2);
    assert_eq!(binding(&result, "before").expression, "\"var(--var-0-0)\"");
    assert!(
        binding(&result, "first")
            .expression
            .contains("var(--var-0-1)")
    );
    assert!(
        binding(&result, "first")
            .expression
            .contains("container-0-2")
    );
    assert!(
        binding(&result, "second")
            .expression
            .contains("var(--var-0-3)")
    );
    assert!(
        binding(&result, "second")
            .expression
            .contains("container-0-4")
    );
    assert_eq!(binding(&result, "after").expression, "\"var(--var-0-5)\"");
    Ok(())
}

#[test]
#[serial]
fn retains_global_effects_and_undefined_when_expression_roots_are_unused() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = "import {globalStyle,createGlobalTheme,createThemeContract,createVar} from '@vanilla-extract/css';globalStyle('body',{margin:0});const vars=createThemeContract({space:null});createGlobalTheme(':root',vars,{space:'8px'});createVar();";
    // When
    let result = run(written("/effects.ts", code), None)?;
    // Then
    assert_eq!(
        result.collected.global_styles,
        vec![
            ("body".into(), "{\"margin\":0}".into()),
            (":root".into(), "{\"--space-0-0\":\"8px\"}".into())
        ]
    );
    let expressions: Vec<_> = result
        .captures
        .iter()
        .filter(|capture| capture.binding.is_none())
        .map(|capture| capture.expression.as_str())
        .collect();
    assert_eq!(
        expressions,
        ["undefined", "undefined", "\"var(--var-0-1)\""]
    );
    Ok(())
}

#[test]
#[serial]
fn captures_final_nested_bindings_when_a_default_allocates_a_native_value() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = "import {createVar,style} from '@vanilla-extract/css';function make(){const value=createVar();return {nested:{value},box:style({margin:value})}}const {nested:{value},box,missing=createVar('default')}=make();const root=style([box,{padding:value,borderWidth:missing}]);const after=createVar();";
    // When
    let result = run(written("/nested.ts", code), None)?;
    // Then
    assert_eq!(binding(&result, "value").expression, "\"var(--var-0-0)\"");
    assert_eq!(
        binding(&result, "missing").expression,
        "\"var(--default-0-1)\""
    );
    assert_eq!(binding(&result, "after").expression, "\"var(--var-0-2)\"");
    assert!(
        result
            .collected
            .styles
            .contains_key(&binding(&result, "box").expression)
    );
    assert_eq!(result.collected.styles.len(), 2);
    assert_eq!(
        binding(&result, "box").root,
        binding(&result, "missing").root
    );
    Ok(())
}

#[test]
#[serial]
fn enforces_only_the_chosen_branch_when_the_other_branch_reads_window_and_a_dynamic_api()
-> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = "import * as ve from '@vanilla-extract/css';const chosen=true;const box=chosen?ve.style({color:'red'}):ve[window.name]({color:window.color});";
    // When
    let result = run(written("/branch.ts", code), None)?;
    // Then
    assert_eq!(result.collected.styles.len(), 1);
    assert!(
        result
            .collected
            .styles
            .contains_key(&binding(&result, "box").expression)
    );
    Ok(())
}

#[test]
#[serial]
fn reserves_all_original_names_when_excluded_bindings_collide_with_generated_names()
-> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = "import {style,keyframes as frames} from '@vanilla-extract/css';const css=window.name,globalCss=window.name,keyframes=window.name,_ve0=window.name,__ve_capture_0__=window.name;const result=(()=>({nested:[style({color:'red'}),frames({from:{opacity:0}})]}))();style({padding:8});";
    // When
    let result = run(written("/hygiene.ts", code), None)?;
    // Then
    for name in result
        .collected
        .styles
        .keys()
        .chain(result.collected.keyframes.keys())
        .chain(result.captures.iter().map(|capture| &capture.name))
    {
        assert!(
            ![
                "css",
                "globalCss",
                "keyframes",
                "_ve0",
                "__ve_capture_0__",
                "result"
            ]
            .contains(&name.as_str())
        );
    }
    let reconstruction = format!(
        "{}{}{}",
        result.emission.header,
        result
            .emission
            .effects
            .iter()
            .map(|(_, code)| code.as_str())
            .collect::<String>(),
        binding(&result, "result").expression
    );
    assert!(reconstruction.contains("_ve0_"), "{reconstruction}");
    assert!(!reconstruction.contains("__style_"), "{reconstruction}");
    assert_eq!(result.collected.references.len(), 3);
    Ok(())
}

#[test]
#[serial]
fn executes_original_api_aliases_when_namespace_and_named_imports_share_the_mock()
-> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = "import {createTheme as theme,style as paint} from '@vanilla-extract/css';import * as ve from '@vanilla-extract/css';const {createVar:variable}=ve;const token=variable();const [name,vars]=theme({space:'8px'});const box=paint({margin:vars.space,padding:token});";
    // When
    let result = run(written("/aliases.ts", code), None)?;
    // Then
    assert_eq!(binding(&result, "token").expression, "\"var(--var-0-0)\"");
    assert_eq!(binding(&result, "name").expression, "\"theme-0-1\"");
    assert!(
        binding(&result, "vars")
            .expression
            .contains("var(--space-0-2)")
    );
    assert_eq!(result.collected.styles.len(), 1);
    Ok(())
}
