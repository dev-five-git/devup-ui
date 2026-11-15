//! `@vanilla-extract/css-utils` compiles to the strings the package builds
//! (VE-05): the expected values are what `@vanilla-extract/css-utils` 0.1.6
//! returns for each expression
use std::collections::HashMap;

use css::{class_map::reset_class_map, file_map::reset_file_map};
use insta::assert_snapshot;
use rstest::rstest;
use serial_test::serial;

use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::{ExtractOption, ImportAlias, extract};

const UTILS: &str = "import { calc } from '@vanilla-extract/css-utils';";

/// Constants a `calc` may read, beside bindings it may not
const CONSTANTS: &str = "const GAP = '4px';\nexport const HALF = `50%`;\nconst SCALE = 2;\nlet loose = '1px';\nconst { picked } = { picked: '1px' };\nconst computed = [1].length;\nexport function f() {}\nexport default 1;";

fn aliases(css_utils: bool) -> ExtractOption {
    let mut import_aliases = HashMap::from([(
        "@vanilla-extract/css".to_string(),
        ImportAlias::NamedToNamed,
    )]);
    if css_utils {
        import_aliases.insert(
            "@vanilla-extract/css-utils".to_string(),
            ImportAlias::NamedToNamed,
        );
    }
    ExtractOption {
        import_aliases,
        ..ExtractOption::default()
    }
}

/// The code `code` of `filename` compiles to, with the `width` it styles,
/// or the error it fails with.
///
/// A call nested in a chained call takes the unoptimized stylesheet pipeline
/// more than the 2 MiB of a test thread, whatever it calls, so it runs on a
/// larger one
fn compiled(filename: &str, code: &str) -> Result<(String, Vec<String>), String> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(8 << 20)
            .spawn_scoped(scope, || compiled_here(filename, code))
            .map_err(|error| error.to_string())?
            .join()
            .map_err(|_| "the extraction panicked".to_string())?
    })
}

fn compiled_here(filename: &str, code: &str) -> Result<(String, Vec<String>), String> {
    reset_class_map();
    reset_file_map();
    let output = extract(filename, code, aliases(true)).map_err(|error| error.to_string())?;
    let mut widths: Vec<String> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "width" => {
                Some(style.value.clone())
            }
            _ => None,
        })
        .collect();
    widths.sort();
    Ok((output.code, widths))
}

fn error(filename: &str, code: &str) -> String {
    compiled(filename, code).err().unwrap_or_default()
}

#[rstest]
#[case("calc.add('1px', '2px')", "calc(1px + 2px)")]
#[case("calc.subtract('100%', '4px', '2px')", "calc(100% - 4px - 2px)")]
#[case("calc.multiply('var(--x)', 2)", "calc(var(--x) * 2)")]
#[case("calc.divide('100%', 3)", "calc(100% / 3)")]
#[case("calc.negate('4px')", "calc(4px * -1)")]
#[case("calc.add(calc.multiply('2px', 3), '1px')", "calc((2px * 3) + 1px)")]
#[case(
    "calc('10px').add('5px').multiply(2).toString()",
    "calc((10px + 5px) * 2)"
)]
#[case("calc('10px').negate().toString()", "calc(10px * -1)")]
#[case(
    "calc('1px').add(2).divide(3, calc.subtract('1px', '-2px')).toString()",
    "calc((1px + 2) / 3 / (1px - -2px))"
)]
#[case(
    "calc(-1.5).subtract(calc('1px').add('2px')).toString()",
    "calc(-1.5 - (1px + 2px))"
)]
#[case("`${calc('1rem').multiply(2)}`", "calc(1rem * 2)")]
#[case("calc.add('var(--calc-gap)', `1px`)", "calc(var(---gap) + 1px)")]
#[case("calc.add(true, 1e21)", "calc(true + 1e+21)")]
#[case("calc.add('1px', '2px').toString()", "calc(1px + 2px)")]
#[case("calc.add(GAP, HALF, SCALE)", "calc(4px + 50% + 2)")]
#[case("(calc as typeof calc).add('1px', '2px')", "calc(1px + 2px)")]
#[serial]
fn calc_gives_the_string_the_package_builds(#[case] expression: &str, #[case] expected: &str) {
    // Outside a stylesheet each call is written as its string
    let (code, widths) = compiled(
        "app.tsx",
        &format!(
            "import {{ css }} from '@devup-ui/react';\n{UTILS}\nexport const joined = [1].join('-');\n{CONSTANTS}\nexport const a = css({{ width: {expression} }});"
        ),
    )
    .unwrap_or_default();
    assert_eq!(widths, [expected], "{expression}\n{code}");
    assert!(!code.contains("css-utils"), "{code}");
    // A stylesheet evaluates the package's own `calc`
    let stylesheet = format!(
        "import {{ style }} from '@vanilla-extract/css';\n{UTILS}\n{CONSTANTS}\nexport const a = style({{ width: {expression} }});"
    );
    let (code, widths) = compiled("theme.css.ts", &stylesheet).unwrap_or_default();
    assert_eq!(widths, [expected], "{expression}\n{code}");
}

#[test]
#[serial]
fn calc_outside_styles_is_written_as_its_string() {
    let compiled = compiled(
        "app.ts",
        "import type { Unused } from '@vanilla-extract/css-utils';
import { calc, type Calc } from '@vanilla-extract/css-utils';
export const gap: typeof calc = calc.add('1px', '2px') as never;
export const half = `${calc('100%').divide(2)} - 1px`;",
    );
    assert_snapshot!(format!("{compiled:#?}"));
}

#[test]
#[serial]
fn calc_in_a_stylesheet_reads_what_the_stylesheet_evaluates() {
    let compiled = compiled(
        "theme.css.ts",
        "import { createTheme, createVar, style } from '@vanilla-extract/css';\nimport * as utils from '@vanilla-extract/css-utils';\nimport { calc } from '@vanilla-extract/css-utils';\nconst gap = createVar();\nexport const [theme, vars] = createTheme({ space: { md: '8px' } });\nexport const a = style({ width: utils.calc(vars.space.md).multiply(2).add(calc.negate(gap)).toString(), vars: { [gap]: '4px' } });",
    );
    assert_snapshot!(format!("{compiled:#?}"));
}

#[rstest]
#[case("calc.add('1px', gap)", "app.tsx:3:47: ")]
#[case("calc.add(picked)", "app.tsx:3:40: ")]
#[case("calc.add(theme.gap)", "app.tsx:3:40: ")]
#[case("calc.add(window)", "app.tsx:3:40: ")]
#[case("calc.add(...parts)", "app.tsx:3:31: ")]
#[case("calc()", "app.tsx:3:31: ")]
#[case("calc('1px', '2px').toString()", "app.tsx:3:31: ")]
#[case("calc.negate()", "app.tsx:3:31: ")]
#[case("calc.percent('1px')", "app.tsx:3:31: ")]
#[case("calc('1px').percent()", "app.tsx:3:31: ")]
#[case("calc('1px').toString('x')", "app.tsx:3:31: ")]
#[case("calc('1px').negate('x').toString()", "app.tsx:3:31: ")]
#[case("calc.add('1px', '2px').trim()", "app.tsx:3:31: ")]
#[case("calc('1px').multiply(2)", "app.tsx:3:31: ")]
#[case("css`width: ${calc('1px')};`", "app.tsx:3:44: ")]
#[case("calc", "app.tsx:3:31: ")]
#[case("calc.add", "app.tsx:3:31: ")]
#[case("[calc].map(String)[0]", "app.tsx:3:32: ")]
#[serial]
fn calc_the_build_cannot_compute_is_a_located_error(
    #[case] expression: &str,
    #[case] location: &str,
) {
    let message = error(
        "app.tsx",
        &format!(
            "import {{ css }} from '@devup-ui/react';\n{UTILS}\nexport const a = css({{ width: {expression} }});\nlet gap = '4px';\nconst {{ picked }} = {{ picked: '1px' }};\nconst theme = {{ gap: '1px' }};\nconst parts = ['1px'];"
        ),
    );
    assert!(message.starts_with(location), "{expression}\n{message}");
    assert!(message.contains("Fix: "), "{expression}\n{message}");
}

#[rstest]
#[case("app.tsx", "import * as utils from '@vanilla-extract/css-utils';")]
#[case("app.tsx", "import utils from '@vanilla-extract/css-utils';")]
#[case("app.tsx", "import { percent } from '@vanilla-extract/css-utils';")]
#[case("app.tsx", "export { calc } from '@vanilla-extract/css-utils';")]
#[case("app.tsx", "const utils = require('@vanilla-extract/css-utils');")]
#[case("theme.css.ts", "import utils from '@vanilla-extract/css-utils';")]
#[case(
    "theme.css.ts",
    "import { percent } from '@vanilla-extract/css-utils';"
)]
#[serial]
fn css_utils_loaded_but_as_calc_by_name_is_a_located_error(
    #[case] filename: &str,
    #[case] load: &str,
) {
    let message = error(
        filename,
        &format!(
            "import {{ style }} from '@vanilla-extract/css';\n{load}\nexport const a = style({{ width: '1px' }});"
        ),
    );
    assert!(
        message.starts_with(&format!("{filename}:2:")),
        "{load}\n{message}"
    );
    assert!(
        message.contains("Fix: import `calc` by name"),
        "{load}\n{message}"
    );
}

#[test]
#[serial]
fn calc_expression_written_as_a_stylesheet_value_is_a_located_error() {
    let message = error(
        "theme.css.ts",
        &format!(
            "import {{ style }} from '@vanilla-extract/css';\n{UTILS}\nexport const a = style({{ width: calc('1px').multiply(2) }});"
        ),
    );
    assert_snapshot!(message);
}

#[test]
#[serial]
fn css_utils_left_unaliased_stays_as_written() {
    reset_class_map();
    reset_file_map();
    let source = format!("{UTILS}\nexport const a = calc.add(gap, '1px');");
    let output = extract("app.tsx", &source, aliases(false)).map_err(|error| error.to_string());
    assert!(output.is_ok_and(|output| output.code.contains("@vanilla-extract/css-utils")));
    let message = extract(
        "theme.css.ts",
        &format!("import {{ style }} from '@vanilla-extract/css';\n{UTILS}\nexport const a = style({{ width: calc.add('1px', '2px') }});"),
        aliases(false),
    )
    .err()
    .map_or_default(|error| error.to_string());
    assert!(message.contains("without a module resolver"), "{message}");
}
