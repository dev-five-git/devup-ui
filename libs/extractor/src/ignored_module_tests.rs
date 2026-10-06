use super::*;
use serial_test::serial;

#[test]
#[serial]
fn ignored_named_values_when_resolved_have_no_dynamic_style_or_dependency()
-> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| Some(ModuleResolution::Ignored);
    for binding in [
        "import { missing as value } from 'empty';",
        "import * as ns from 'empty'; const value = ns.missing;",
        "import def from 'empty'; const value = def.missing;",
    ] {
        let output = extract_with_modules(
            "/src/ignored.tsx",
            &format!(
                "import {{Box}} from '@devup-ui/react'; {binding} export const view = <Box color={{value}} bg='red'/>;"
            ),
            ExtractOption::default(),
            false,
            &resolver,
        )?;
        assert_eq!(output.dependencies, Vec::<String>::new());
        assert!(!output.code.contains("--"), "{}", output.code);
    }
    Ok(())
}

#[test]
#[serial]
fn ignored_stylesheet_bindings_when_evaluated_are_empty_commonjs() -> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| Some(ModuleResolution::Ignored);
    let output = extract_with_modules(
        "/src/ignored.css.ts",
        "import {style} from '@devup-ui/react'; import def, {missing} from 'empty'; import * as ns from 'empty'; function color() { if (missing !== undefined || Object.keys(ns).length || Object.keys(def).length) throw new Error('bad interop'); return 'red'; } export const cls = style({color: color()});",
        ExtractOption::default(),
        false,
        &resolver,
    )?;
    assert_eq!(output.dependencies, Vec::<String>::new());
    assert!(!output.code.contains("style("));
    assert!(!output.code.contains("--"));
    Ok(())
}

#[test]
#[serial]
fn ignored_constants_when_required_or_reexported_remain_known() -> Result<(), Box<dyn Error>> {
    let resolver = |specifier: &str, _: &str| {
        match specifier {
        "empty" => Some(ModuleResolution::Ignored),
        "barrel" => Some(ModuleResolution::Resolved(ResolvedModule {
            path: "/src/barrel.js".to_string(),
            code: "export {missing as value} from 'empty'; export * as ns from 'empty'; export * from 'empty';".to_string(),
            source_type: None,
        })),
        "cjs" => Some(ModuleResolution::Resolved(ResolvedModule {
            path: "/src/cjs.js".to_string(),
            code: "module.exports = require('empty');".to_string(),
            source_type: None,
        })),
        "cjsProps" => Some(ModuleResolution::Resolved(ResolvedModule {
            path: "/src/cjsProps.js".to_string(),
            code: "const {missing} = require('empty'); exports.value = missing; exports.other = require('empty').absent;".to_string(),
            source_type: None,
        })),
        _ => None,
    }
    };
    for (binding, dependencies) in [
        ("import {value} from 'barrel';", vec!["/src/barrel.js"]),
        (
            "import {ns} from 'barrel'; const value = ns.missing;",
            vec!["/src/barrel.js"],
        ),
        (
            "import def from 'cjs'; const value = def.missing;",
            vec!["/src/cjs.js"],
        ),
        ("import {value} from 'cjsProps';", vec!["/src/cjsProps.js"]),
        (
            "import {other as value} from 'cjsProps';",
            vec!["/src/cjsProps.js"],
        ),
    ] {
        let output = extract_with_modules(
            "/src/entry.tsx",
            &format!(
                "import {{Box}} from '@devup-ui/react'; {binding} export const view = <Box color={{value}}/>;"
            ),
            ExtractOption::default(),
            false,
            &resolver,
        )?;
        assert_eq!(output.dependencies, dependencies);
        assert!(!output.code.contains("--"), "{}", output.code);
    }
    Ok(())
}

#[test]
#[serial]
fn ignored_object_when_mutated_is_not_inlined_as_empty() -> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| Some(ModuleResolution::Ignored);
    for binding in ["import def from 'empty';", "import * as def from 'empty';"] {
        let output = extract_with_modules(
            "/src/mutated.tsx",
            &format!(
                "import {{Box}} from '@devup-ui/react'; {binding} def.color = 'blue'; export const view = <Box color={{def.color}}/>;"
            ),
            ExtractOption::default(),
            false,
            &resolver,
        )?;
        assert!(output.code.contains("--"));
        assert_eq!(output.dependencies, Vec::<String>::new());
    }
    Ok(())
}

#[test]
#[serial]
fn real_commonjs_when_empty_keeps_inherited_reads() -> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "/src/tokens.js".to_string(),
            code: "module.exports = {};".to_string(),
            source_type: None,
        }))
    };
    let output = extract_with_modules(
        "/src/real.tsx",
        "import {Box} from '@devup-ui/react'; import def from 'tokens'; export const view = <Box color={def.constructor === Object ? 'red' : 'blue'}/>;",
        ExtractOption::default(),
        false,
        &resolver,
    )?;
    assert!(output.code.contains("def.constructor"), "{}", output.code);
    assert_eq!(output.dependencies, vec!["/src/tokens.js"]);
    Ok(())
}

#[test]
#[serial]
fn ignored_missing_when_undefined_is_shadowed_keeps_primitive_argument()
-> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| Some(ModuleResolution::Ignored);
    let output = extract_with_modules(
        "/src/shadowed.tsx",
        "import {Box} from '@devup-ui/react'; import {missing} from 'empty'; export const View = (undefined, read) => <Box color={read(missing)}/>;",
        ExtractOption::default(),
        false,
        &resolver,
    )?;
    assert!(output.code.contains("read(void 0)"), "{}", output.code);
    assert_eq!(output.dependencies, Vec::<String>::new());
    Ok(())
}

#[test]
#[serial]
fn unresolved_require_when_dynamic_or_shadowed_is_not_an_ignored_constant()
-> Result<(), Box<dyn Error>> {
    for code in [
        "module.exports = require(name);",
        "module.exports = require('missing');",
        "const require = () => ({}); module.exports = require('empty');",
    ] {
        let resolver = |specifier: &str, _: &str| match specifier {
            "tokens" => Some(ModuleResolution::Resolved(ResolvedModule {
                path: "/src/tokens.js".to_string(),
                code: code.to_string(),
                source_type: None,
            })),
            "empty" => Some(ModuleResolution::Ignored),
            _ => None,
        };
        let output = extract_with_modules(
            "/src/unresolved.tsx",
            "import {Box} from '@devup-ui/react'; import value from 'tokens'; export const view = <Box color={value.missing}/>;",
            ExtractOption::default(),
            false,
            &resolver,
        )?;
        assert!(output.code.contains("--"));
    }
    Ok(())
}
