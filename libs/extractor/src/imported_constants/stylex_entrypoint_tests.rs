use serial_test::serial;

use crate::{ExtractOption, ExtractStyleValue, ResolvedModule};

#[test]
#[serial]
fn imported_constants_follow_defining_module_when_public_entrypoints_are_used() {
    let entries = [
        ("import * as sx from '@stylexjs/stylex';", "sx"),
        ("import * as sx from '@devup-ui/react/stylex';", "sx"),
        ("import { stylex as sx } from '@devup-ui/react';", "sx"),
        ("import * as devup from '@devup-ui/react';", "devup.stylex"),
        ("const sx = require('@devup-ui/react/stylex');", "sx"),
        ("const { stylex: sx } = require('@devup-ui/react');", "sx"),
    ];
    for (import, namespace) in entries {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        let module = format!(
            "{import}\nexport const tokens = {namespace}.defineConsts({{ width: '10px' }});\nexport const numericWidth = 10;\nexport const fallback = {namespace}.positionTry({{ width: tokens.width }});"
        );
        let resolver = move |specifier: &str, _: &str| {
            (specifier == "./tokens").then(|| ResolvedModule {
                path: "/src/tokens.ts".into(),
                code: module.clone(),
            })
        };
        let output = crate::extract_with_modules("/src/App.ts", "import { stylex as sx } from '@devup-ui/react';\nimport { tokens, fallback, numericWidth } from './tokens';\nexport const styles = sx.create({ item: { width: tokens.width, height: numericWidth, positionTryFallbacks: fallback } });", ExtractOption { import_main_css: false, ..ExtractOption::default() }, false, &resolver).unwrap_or_else(|error| panic!("{error}"));
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "width" && style.value == "10px")), "{:?}", output.styles);
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "height" && style.value == "10px")), "{:?}", output.styles);
        let expected =
            crate::stylex::transitions::TransitionRules::Position("width:10px;".to_string())
                .compile("/src/tokens.ts")
                .0;
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "position-try-fallbacks" && style.value == expected)), "{}", output.code);
    }
}

#[test]
#[serial]
fn variable_and_theme_references_agree_when_custom_root_package_is_configured() {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./tokens").then(|| ResolvedModule {
            path: "/src/tokens.ts".into(),
            code: "import { stylex as sx } from '@custom/ui'; export const vars = sx.defineVars({ color: 'red' }); export const theme = sx.createTheme(vars, { color: 'blue' });".into(),
        })
    };
    let output = crate::extract_with_modules("/src/App.ts", "import * as sx from '@custom/ui/stylex'; import { vars, theme } from './tokens'; const styles = sx.create({ item: { color: vars.color } }); export const props = sx.props(theme, styles.item);", ExtractOption { package: "@custom/ui".into(), import_main_css: false, ..ExtractOption::default() }, false, &resolver).unwrap_or_else(|error| panic!("{error}"));
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "color" && style.value.starts_with("var(--"))), "{}", output.code);
    assert!(!output.code.contains("sx.props("), "{}", output.code);
}

#[test]
#[serial]
fn required_variable_groups_can_create_themes_when_dependency_bindings_are_known() {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./tokens").then(|| ResolvedModule {
            path: "/src/tokens.ts".into(),
            code: "const sx = require('@devup-ui/react/stylex'); export const vars = sx.defineVars({ color: 'red' });".into(),
        })
    };
    let output = crate::extract_with_modules("/src/App.ts", "const sx = require('@devup-ui/react/stylex'); const { vars } = require('./tokens'); export const theme = sx.createTheme(vars, { color: 'blue' });", ExtractOption { import_main_css: false, ..ExtractOption::default() }, false, &resolver).unwrap_or_else(|error| panic!("{error}"));
    assert!(!output.code.contains("createTheme("), "{}", output.code);
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Css(style) if style.css.contains("--sxvar-2f7372632f746f6b656e732e7473-636f6c6f72-:blue"))), "{:?}", output.styles);
}
