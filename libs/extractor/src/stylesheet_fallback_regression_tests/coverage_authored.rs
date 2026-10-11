use rstest::rstest;
use serial_test::serial;

use super::{TestResult, observe, option, reset};
use crate::vanilla_extract::{Stylesheet, execute_authored};
use crate::{ResolvedModule, extract_with_modules, extract_without_source_map};

#[rstest]
#[case("let alias=value;", "value", 11)]
#[case("var alias=value;", "value", 11)]
#[case("const {alias}={ alias:value };", "{ alias:value }", 15)]
#[serial]
fn retained_state_rejects_inexact_alias_setup(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] declaration: &str,
    #[case] initializer: &str,
    #[case] column: usize,
) -> TestResult {
    // Given
    reset();
    let path = format!("/alias-setup.{suffix}");
    let source = format!(
        "import {{style}} from '@devup-ui/react';\nconst value={{}};\n{declaration}\nalias.self=value;\nexport {{value as loop}};\nexport const box=style({{color:'blue'}});"
    );
    // When
    let result = extract_without_source_map(&path, &source, option());
    // Then
    let error = result
        .err()
        .ok_or("inexact alias was accepted")?
        .to_string();
    assert!(error.starts_with(&format!("{path}:3:{column}:")), "{error}");
    assert!(error.contains(&format!("ordinary alias `alias` cannot use `{initializer}` at build time: retained-state setup is not an exact const binding alias")), "{error}");
    assert!(
        error.contains("Fix: use a const alias of the retained binding before mutating it"),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[serial]
fn unrelated_callable_candidates_do_not_change_retained_cycles(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@devup-ui/react';function unused(){return 9}const value={};value.self=value;export {value as loop};export const box=style({color:'blue'});";
    // When
    let output = extract_without_source_map(&format!("/candidate.{suffix}"), source, option())?;
    // Then
    assert!(!output.code.contains("unused"), "{}", output.code);
    observe(&output, "exports.loop.self===exports.loop")
}

#[rstest]
#[case("import fn from './dep';", "fn", "import fn from \"./dep\";")]
#[case(
    "import * as ns from './dep';",
    "ns.fn",
    "import * as ns from \"./dep\";"
)]
#[case(
    "import fn, {fn as other} from './dep';",
    "fn(other())",
    "import { fn as other } from \"./dep\";"
)]
#[serial]
fn standalone_imports_survive_when_authored_helpers_read_them(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] import: &str,
    #[case] read: &str,
    #[case] expected: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@devup-ui/react';\n{import}\nexport const help=()=>{read};\nexport const box=style({{color:'blue'}});"
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./dep").then(|| ResolvedModule {
            path: "/dep.ts".into(),
            code: "export function fn(){return 13}export default fn;".into(),
        })
    };
    // When
    let output = extract_with_modules(
        &format!("/imports.{suffix}"),
        &source,
        option(),
        false,
        &resolver,
    )?;
    // Then
    assert!(output.code.contains(expected), "{}", output.code);
    assert!(output.code.contains("export const help"), "{}", output.code);
    assert!(output.code.contains(read), "{}", output.code);
    Ok(())
}

#[test]
#[serial]
fn type_import_declarations_do_not_survive_runtime_fallbacks() -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@devup-ui/react';import type {Shape} from './types';export const help=(value:Shape)=>value;export const box=style({color:'blue'});";
    // When
    let output = extract_without_source_map("/types.css.ts", source, option())?;
    // Then
    assert!(!output.code.contains("./types"), "{}", output.code);
    assert!(
        output
            .code
            .contains("export const help = (value: Shape) => value;"),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn imported_callable_export_has_no_local_declaration_to_restore() -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@devup-ui/react';import {fn} from './dep';export {fn};export const box=style({color:'blue'});";
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./dep").then(|| ResolvedModule {
            path: "/dep.ts".into(),
            code: "export function fn(){return 13}".into(),
        })
    };
    // When
    let output = extract_with_modules("/imported.css.ts", source, option(), false, &resolver)?;
    // Then
    assert!(
        output.code.contains("import { fn } from \"./dep\";"),
        "{}",
        output.code
    );
    assert!(output.code.contains("export { fn };"), "{}", output.code);
    assert!(!output.code.contains("function fn"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case("import {useTheme} from '@devup-ui/react';", "useTheme")]
#[serial]
fn erased_or_noncompiler_import_is_rejected_at_the_fallback_read(
    #[case] import: &str,
    #[case] name: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!("{import}\nexport const help=()=>{name};");
    // When
    let result = execute_authored(
        Stylesheet {
            filename: "/erased.css.ts",
            code: &source,
            source: &source,
            edits: &[],
        },
        &option(),
        None,
    );
    // Then
    let error = result.err().ok_or("invalid import read was accepted")?;
    assert!(error.starts_with("/erased.css.ts:2:23:"), "{error}");
    assert!(
        error.contains(&format!(
            "authored fallback references binding `{name}` removed by stylesheet compilation"
        )),
        "{error}"
    );
    assert!(error.contains("Fix: make the export self-contained or move its runtime dependency to an ordinary module"), "{error}");
    Ok(())
}

#[rstest]
#[case("css`color:blue`", "css")]
#[case("<Box></Box>", "Box")]
#[serial]
fn compiler_aliases_remain_safe_in_deferred_tags_and_jsx(
    #[case] expression: &str,
    #[case] name: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{css,Box}} from '@devup-ui/react';export const opaque=(...args)=>css(...args);export const help=()=>{expression};"
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "react/jsx-runtime").then(|| ResolvedModule {
            path: "/jsx-runtime.js".into(),
            code: "export function jsx(type,props){return { type, props }}".into(),
        })
    };
    // When
    let (_, _, output) = execute_authored(
        Stylesheet {
            filename: "/compiler.css.tsx",
            code: &source,
            source: &source,
            edits: &[],
        },
        &option(),
        Some(&resolver),
    )?;
    // Then
    assert!(
        output
            .code
            .contains(&format!("export const help=()=>{expression};")),
        "{}",
        output.code
    );
    assert!(
        output.code.lines().any(|line| line.starts_with("import { ")
            && line.contains(name)
            && line.ends_with("from '@devup-ui/react';")),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn compiler_type_reference_does_not_become_a_runtime_dependency() -> TestResult {
    // Given
    reset();
    let source =
        "import {css} from '@devup-ui/react';export const help=(value:typeof css)=>css(...value);";
    // When
    let (_, _, output) = execute_authored(
        Stylesheet {
            filename: "/compiler-type.css.ts",
            code: source,
            source,
            edits: &[],
        },
        &option(),
        None,
    )?;
    // Then
    assert!(output.code.contains("typeof css"), "{}", output.code);
    assert!(
        output
            .code
            .contains("import { css } from '@devup-ui/react';"),
        "{}",
        output.code
    );
    assert!(output.code.contains("export const help"), "{}", output.code);
    Ok(())
}
