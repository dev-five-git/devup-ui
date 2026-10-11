use super::*;
use crate::vanilla_extract::collected_styles_to_code_with_keyframes;
use crate::{ExtractStyleValue, ResolvedModule};

#[test]
#[serial]
fn retains_actual_imported_vars_base_atoms_and_css_edges_when_a_stylesheet_produces_them()
-> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |source: &str, _: &str| {
        match source {
        "./theme.css" => Some(ResolvedModule {
            path: "/theme.css.ts".into(),
            code: "import {createTheme,style} from '@vanilla-extract/css';import './reset.css';export const [theme,vars]=createTheme({space:'8px'});export const base=style({color:'red',padding:8});".into(),
        }),
        "./reset.css" => Some(ResolvedModule { path: "/reset.css".into(), code: "body{margin:0}".into() }),
        "./tokens" => Some(ResolvedModule { path: "/tokens.ts".into(), code: "export const color='blue';".into() }),
        _ => None,
    }
    };
    let code = "import {style,globalStyle,createVar} from '@vanilla-extract/css';import {vars,base} from './theme.css';import {color} from './tokens';const inherited=(()=>{globalStyle(`${base}:hover`,{margin:vars.space});return base})();const box=style([base,{color,margin:vars.space}]);const own=createVar();const host=window.document;";
    // When
    let result = run(written("/consumer.ts", code), Some(&resolver))?;
    // Then
    let base: String = serde_json::from_str(&binding(&result, "inherited").expression)
        .map_err(|error| error.to_string())?;
    let atoms: Vec<_> = base
        .split_whitespace()
        .filter_map(|class| result.imports.atoms.get(class))
        .flatten()
        .collect();
    assert!(
        atoms.iter().any(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property() == "color" && style.value() == "red"))
    );
    assert!(atoms.iter().any(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px")));
    assert!(result.imports.dependencies.contains("/theme.css.ts"));
    assert!(result.imports.dependencies.contains("/tokens.ts"));
    assert!(
        result
            .imports
            .kept_imports
            .iter()
            .any(|source| source == "./theme.css")
    );
    assert!(
        result
            .imports
            .kept_imports
            .iter()
            .any(|source| source.ends_with("reset.css"))
    );
    let lowered = collected_styles_to_code_with_keyframes(
        &result.collected,
        "@devup-ui/react",
        &rustc_hash::FxHashMap::default(),
    );
    assert!(lowered.contains("var(--space-0-1)"));
    assert!(lowered.contains("blue"));
    assert!(lowered.contains(&base));
    let number = css::file_map::get_file_num_by_filename("/consumer.ts");
    assert_eq!(
        binding(&result, "own").expression,
        format!("\"var(--var-{number}-0)\"")
    );
    Ok(())
}

#[test]
#[serial]
fn imports_mixed_producers_without_executing_their_preserved_source() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |source: &str, _: &str| {
        (source == "./producer").then(|| ResolvedModule {
        path: "/producer.tsx".into(),
        code: "import {style} from '@vanilla-extract/css';export const base=style({color:'red'});throw new Error('must not run');export const view=<div/>;".into(),
    })
    };
    let code = "import {style} from '@vanilla-extract/css';import {base} from './producer';const box=style([base,{color:'blue'}]);";
    // When
    let result = run(written("/consumer.ts", code), Some(&resolver))?;
    // Then
    let lowered = collected_styles_to_code_with_keyframes(
        &result.collected,
        "@devup-ui/react",
        &rustc_hash::FxHashMap::default(),
    );
    assert!(lowered.contains("blue"), "{lowered}");
    assert!(!lowered.contains("red"), "{lowered}");
    assert!(result.imports.dependencies.contains("/producer.tsx"));
    Ok(())
}

#[test]
#[serial]
fn propagates_selector_reference_metadata_when_imported_theme_classes_are_targets()
-> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |source: &str, _: &str| {
        (source == "./theme.css").then(|| ResolvedModule {
        path: "/theme.css.ts".into(),
        code: "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});".into(),
    })
    };
    let code = "import {globalStyle} from '@vanilla-extract/css';import {theme,vars} from './theme.css';globalStyle(`${theme} &`,{margin:vars.space});";
    // When
    let result = run(written("/selector.ts", code), Some(&resolver))?;
    // Then
    assert_eq!(result.collected.global_styles[0].0, ".theme-0-0 &");
    assert_eq!(
        result.collected.global_styles[0].1,
        "{\"margin\":\"var(--space-0-1)\"}"
    );
    Ok(())
}
