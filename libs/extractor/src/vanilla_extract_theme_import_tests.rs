//! An ordinary module reading what a vanilla-extract stylesheet exports.

use crate::{ExtractOption, ExtractStyleValue, ImportAlias, ResolvedModule, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

const THEME: &str = "import { createGlobalTheme, createTheme, createThemeContract, createVar, style } from '@vanilla-extract/css';
import { BRAND } from './tokens';
export const vars = createThemeContract({ colors: { bg: null }, space: null });
export const light = createTheme(vars, { colors: { bg: 'white' }, space: '8px' });
export const [themeClass, tuple] = createTheme({ space: { md: '8px' } });
export const global = createGlobalTheme(':root', { brand: BRAND });
export const accent = createVar();
export const base = style({ padding: 4 });
";

const BROKEN: &str = "import { createThemeContract } from '@vanilla-extract/css';
export const vars = createThemeContract(undefined.space);
";

const CYCLE: &str = "import { createThemeContract } from '@vanilla-extract/css';
import { SIZE } from './size';
export const vars = createThemeContract({ space: null });
export const size = SIZE;
";

fn option() -> ExtractOption {
    ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

fn resolver(specifier: &str, _: &str) -> Option<ResolvedModule> {
    let (path, code) = match specifier {
        "./theme.css" => ("/src/theme.css.ts", THEME),
        "./tokens" => ("/src/tokens.ts", "export const BRAND = 'blue';"),
        "./broken.css" => ("/src/broken.css.ts", BROKEN),
        "./cycle.css" => ("/src/cycle.css.ts", CYCLE),
        "./size" => (
            "/src/size.ts",
            "import { vars } from './cycle.css';\nexport const SIZE = '4px';\nexport const read = () => vars;\n",
        ),
        _ => return None,
    };
    Some(ResolvedModule {
        path: path.to_string(),
        code: code.to_string(),
    })
}

fn extract(filename: &str, code: &str) -> Result<crate::ExtractOutput, String> {
    extract_with_modules(filename, code, option(), false, &resolver)
        .map_err(|error| error.to_string())
}

fn statics(output: &crate::ExtractOutput) -> Vec<(String, String)> {
    let mut statics: Vec<(String, String)> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property().to_string(), style.value().to_string()))
            }
            _ => None,
        })
        .collect();
    statics.sort();
    statics
}

#[rstest]
#[case::contract(
    "import { style } from '@vanilla-extract/css';\nimport { vars } from './theme.css';\nexport const button = style({ margin: vars.space, color: vars.colors.bg });\n",
    &[("color", "var(--colors-bg-0-0)"), ("margin", "var(--space-0-1)")]
)]
#[case::theme_tuple_global_theme_and_var(
    "import { style } from '@vanilla-extract/css';\nimport { accent, global, tuple } from './theme.css';\nexport const button = style({ padding: tuple.space.md, color: global.brand, background: accent });\n",
    &[("background", "var(--var-0-6)"), ("color", "var(--brand-0-5)"), ("padding", "var(--space-md-0-4)")]
)]
#[case::devup_css_through_a_namespace(
    "import { css } from '@devup-ui/react';\nimport * as theme from './theme.css';\nexport const button = css({ p: theme.tuple.space.md });\n",
    &[("padding", "var(--space-md-0-4)")]
)]
#[case::jsx_props(
    "import { Box } from '@devup-ui/react';\nimport { vars } from './theme.css';\nexport const Button = () => <Box bg={vars.colors.bg} p={vars.space} />;\n",
    &[("background", "var(--colors-bg-0-0)"), ("padding", "var(--space-0-1)")]
)]
#[case::composed_class(
    "import { style } from '@vanilla-extract/css';\nimport { base } from './theme.css';\nexport const button = style([base, { color: 'red' }]);\n",
    &[("color", "red")]
)]
#[serial]
fn ordinary_module_reads_the_names_the_stylesheet_css_uses(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result<(), String> {
    // Given: the stylesheet extracted the way the bundler extracts it
    reset_class_map();
    reset_file_map();
    let stylesheet = extract("/src/theme.css.ts", THEME)?;
    // When
    let output = extract("/src/button.tsx", code)?;
    // Then: every name read is one the stylesheet itself exports
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(property, value)| ((*property).to_string(), (*value).to_string()))
        .collect();
    assert_eq!(statics(&output), expected);
    for (_, value) in &expected {
        assert!(
            !value.starts_with("var(") || stylesheet.code.contains(value.as_str()),
            "{value} is not in {}",
            stylesheet.code
        );
    }
    assert!(
        output.code.contains("from \"./theme.css\""),
        "{}",
        output.code
    );
    assert!(
        output
            .dependencies
            .iter()
            .any(|file| file == "/src/theme.css.ts"),
        "{:?}",
        output.dependencies
    );
    Ok(())
}

#[test]
#[serial]
fn composed_class_is_the_one_the_stylesheet_exports() -> Result<(), String> {
    // Given
    reset_class_map();
    reset_file_map();
    let stylesheet = extract("/src/theme.css.ts", THEME)?;
    let base = stylesheet
        .code
        .split("export const base = \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .ok_or_else(|| stylesheet.code.clone())?
        .to_string();
    // When
    let output = extract(
        "/src/button.ts",
        "import { style } from '@vanilla-extract/css';\nimport { base } from './theme.css';\nexport const button = style([base, { color: 'red' }]);\n",
    )?;
    // Then
    assert!(
        output
            .code
            .contains(&format!("export const button = \"{base} ")),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn importer_depends_on_what_the_stylesheet_reads() -> Result<(), String> {
    // Given
    reset_class_map();
    reset_file_map();
    // When
    let output = extract(
        "/src/button.ts",
        "import { style } from '@vanilla-extract/css';\nimport { global } from './theme.css';\nexport const button = style({ color: global.brand });\n",
    )?;
    // Then
    assert!(
        output
            .dependencies
            .iter()
            .any(|file| file == "/src/tokens.ts"),
        "{:?}",
        output.dependencies
    );
    Ok(())
}

#[test]
#[serial]
fn stylesheet_that_fails_stays_a_located_error() {
    // Given
    reset_class_map();
    reset_file_map();
    // When
    let error = extract(
        "/src/button.ts",
        "import { style } from '@vanilla-extract/css';\nimport { vars } from './broken.css';\nexport const button = style({ margin: vars.space });\n",
    );
    // Then
    assert_eq!(
        error.err().as_deref(),
        Some(
            "/src/button.ts:3:23: `css()` cannot use `vars.space` at build time: its values must be literals, theme tokens or constants, or be computed from them"
        )
    );
}

#[test]
#[serial]
fn stylesheet_importing_its_importer_still_compiles() -> Result<(), String> {
    // Given
    reset_class_map();
    reset_file_map();
    // When
    let output = extract(
        "/src/size.tsx",
        "import { css } from '@devup-ui/react';\nimport { vars, size } from './cycle.css';\nexport const box = css({ p: vars.space, m: size });\n",
    )?;
    // Then
    assert_eq!(
        statics(&output),
        [
            ("margin".to_string(), "4px".to_string()),
            ("padding".to_string(), "var(--space-1-0)".to_string())
        ]
    );
    Ok(())
}

const DOCUMENTED_THEME: &str =
    "import { createTheme, createThemeContract, style } from '@vanilla-extract/css'

export const vars = createThemeContract({ colors: { bg: null }, space: null })
export const light = createTheme(vars, {
  colors: { bg: 'white' },
  space: '8px',
})
export const box = style({ background: vars.colors.bg })
export const base = style({ padding: 8 })
";

fn documented(specifier: &str, _: &str) -> Option<ResolvedModule> {
    let (path, code) = match specifier {
        "./theme.css" => ("/src/theme.css.ts", DOCUMENTED_THEME),
        "./tokens" => ("/src/tokens.ts", "export const PRIMARY = 'blue'\n"),
        _ => return None,
    };
    Some(ResolvedModule {
        path: path.to_string(),
        code: code.to_string(),
    })
}

#[test]
#[serial]
fn documented_ordinary_module_reading_a_stylesheet_compiles() -> Result<(), String> {
    // Given: the migration guide's `button.ts`
    reset_class_map();
    reset_file_map();
    let option = ExtractOption {
        import_main_css: false,
        ..option()
    };
    extract_with_modules(
        "/src/theme.css.ts",
        DOCUMENTED_THEME,
        option.clone(),
        false,
        &documented,
    )
    .map_err(|error| error.to_string())?;
    // When
    let output = extract_with_modules(
        "/src/button.ts",
        "import { style } from '@vanilla-extract/css'\n\nimport { base, vars } from './theme.css'\nimport { PRIMARY } from './tokens'\n\nexport const button = style([base, { color: PRIMARY, margin: vars.space }])\n",
        option,
        false,
        &documented,
    )
    .map_err(|error| error.to_string())?;
    // Then
    insta::assert_snapshot!(output.code);
    assert_eq!(
        statics(&output),
        [
            ("color".to_string(), "blue".to_string()),
            ("margin".to_string(), "var(--space-0-1)".to_string())
        ]
    );
    Ok(())
}
