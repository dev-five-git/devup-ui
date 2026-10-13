use rstest::rstest;
use serial_test::serial;

use super::{TestResult, option, reset};
use crate::vanilla_extract::{Stylesheet, execute_authored};
use crate::{ResolvedModule, extract_with_modules};

#[rstest]
#[serial]
fn coverage_unit_i_restores_callable_star_exports(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = "export * from './other.css';export const own=17;";
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./other.css").then(|| ResolvedModule {
            path: "/other.css.ts".into(),
            code: "export function read(){return 23}export const other=19;export default 29;"
                .into(),
        })
    };
    // When
    let (collected, imports, authored) = execute_authored(
        Stylesheet {
            filename: &format!("/star.{suffix}"),
            code: source,
            source,
            edits: &[],
        },
        &option(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(authored.code, "export * from './other.css';");
    assert!(
        collected
            .constant_exports
            .iter()
            .any(|(name, value)| name == "own" && value == "17")
    );
    assert!(
        collected
            .export_aliases
            .iter()
            .any(|(_, name, value)| name == "other" && value == "19")
    );
    assert!(
        !collected
            .export_aliases
            .iter()
            .any(|(_, name, _)| name == "default")
    );
    assert!(
        imports
            .dependencies
            .iter()
            .any(|path| path == "/other.css.ts")
    );
    Ok(())
}

#[rstest]
#[case("module.exports={ ...other };", "module", "module.exports")]
#[case("exports.read=other.read;", "exports", "exports.read")]
#[serial]
fn coverage_unit_i_commonjs_entry_is_rejected_before_authored_export_collection(
    #[case] export: &str,
    #[case] binding: &str,
    #[case] read: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "const {{style}}=require('@devup-ui/react');const other=require('./other');\n{export}\nconst box=style({{color:'blue'}});"
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./other").then(|| ResolvedModule {
            path: "/other.js".into(),
            code: "exports.read=()=>23;".into(),
        })
    };
    // When
    let result = execute_authored(
        Stylesheet {
            filename: "/missing.css.js",
            code: &source,
            source: &source,
            edits: &[],
        },
        &option(),
        Some(&resolver),
    );
    // Then
    let error = result
        .err()
        .ok_or("CommonJS entry export was silently accepted")?;
    assert!(error.starts_with("/missing.css.js:2:1:"), "{error}");
    assert!(
        error.contains(&format!(
            "ReferenceError: `{binding}` cannot be read at build time"
        )),
        "{error}"
    );
    assert!(
        error.contains(&format!(
            "Fix: use a literal or a CSS variable for `{binding}`"
        )),
        "{error}"
    );
    assert!(
        error.contains(&format!(
            "/missing.css.js:2:1: native styling cannot use `{read}` at build time"
        )),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case("")]
#[case("undefined")]
#[case("7")]
#[serial]
fn coverage_unit_i_empty_style_operand_emits_an_empty_class(#[case] operand: &str) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';export const empty=style({operand});"
    );
    // When
    let output = extract_with_modules("/empty.css.ts", &source, option(), false, &|_, _| None)?;
    // Then
    assert_eq!(output.styles.len(), 0);
    assert!(
        output.code.contains("export const empty = \"\";"),
        "{}",
        output.code
    );
    assert!(!output.code.contains("style("), "{}", output.code);
    Ok(())
}

#[rstest]
#[case("module.exports={ ...other };")]
#[case("exports.read=other.read;exports.other=other.other;exports.default=29;")]
#[serial]
fn coverage_unit_i_star_collects_commonjs_spread_exports(#[case] export: &str) -> TestResult {
    // Given
    reset();
    let source = "export * from './other';export const own=17;";
    let module = format!("const other={{read:()=>23,other:19,default:29}};{export}");
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./other").then(|| ResolvedModule {
            path: "/other.js".into(),
            code: module.clone(),
        })
    };
    // When
    let (collected, _, authored) = execute_authored(
        Stylesheet {
            filename: "/commonjs-star.css.js",
            code: source,
            source,
            edits: &[],
        },
        &option(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(authored.code, "export * from './other';");
    assert!(
        collected
            .export_aliases
            .iter()
            .any(|(_, name, value)| name == "other" && value == "19")
    );
    assert!(
        !collected
            .export_aliases
            .iter()
            .any(|(_, name, _)| name == "default")
    );
    Ok(())
}
