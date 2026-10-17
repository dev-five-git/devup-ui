use super::*;
use serial_test::serial;

#[test]
#[serial]
fn extracts_real_filename_when_compiled_mode_is_supplied() -> Result<(), Box<dyn Error>> {
    let code = "import {Box} from '@devup-ui/react'; export const view = <Box bg='red'/>;";
    let output = extract_with_source_type(
        "page.mdown",
        code,
        ExtractOption::default(),
        true,
        None,
        Some(ExtractSourceType::CompiledMdx),
    )?;
    assert!(output.map.is_some_and(|map| map.contains("page.mdown")));
    assert!(!output.code.contains("<Box"));
    Ok(())
}

#[test]
#[serial]
fn derives_child_language_when_root_is_compiled_mdx() -> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "token.ts".to_string(),
            code: "export enum Colors { PRIMARY = 'red' }".to_string(),
            source_type: None,
        }))
    };
    let output = extract_with_source_type(
        "page.mdown",
        "import {Box} from '@devup-ui/react'; import {Colors} from './token'; export const view = <Box color={Colors.PRIMARY}/>;",
        ExtractOption::default(),
        false,
        Some(&resolver),
        Some(ExtractSourceType::CompiledMdx),
    )?;
    assert!(!output.code.contains("--"));
    assert_eq!(output.dependencies, ["token.ts"]);
    Ok(())
}

#[test]
#[serial]
fn rejects_child_extension_when_constants_would_fall_back() {
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "token.custom".to_string(),
            code: "export const PRIMARY = 'red';".to_string(),
            source_type: None,
        }))
    };
    let result = extract_with_source_type(
        "page.tsx",
        "import {Box} from '@devup-ui/react'; import {PRIMARY} from './token'; import {OTHER} from './other'; export const view = <Box color={PRIMARY} bg={OTHER}/>;",
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
    );
    assert!(result.is_err_and(|error| error.to_string().starts_with("token.custom:1:1:")));
}

#[test]
#[serial]
fn evaluates_stylesheet_when_imported_compiled_jsx_has_trailing_constants()
-> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "token.mdown".to_string(),
            code: "export function view() { return <div/>; } export const PRIMARY = 'blue';"
                .to_string(),
            source_type: Some(ExtractSourceType::CompiledMdx),
        }))
    };
    let output = extract_with_source_type(
        "theme.css.ts",
        "import {css} from '@devup-ui/react'; import {PRIMARY} from './token'; export const cls = css({color: PRIMARY});",
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
    )?;
    assert!(!output.code.contains("--"));
    assert_eq!(output.dependencies, ["token.mdown"]);
    Ok(())
}

#[test]
#[serial]
fn evaluates_generated_values_when_root_keeps_custom_filename() -> Result<(), Box<dyn Error>> {
    let output = extract_with_source_type(
        "page.mdown",
        "import {css} from '@devup-ui/react'; const twice = (x) => x * 2; export const cls = css({width: twice(10)});",
        ExtractOption::default(),
        false,
        None,
        Some(ExtractSourceType::CompiledMdx),
    )?;
    assert!(!output.code.contains("css("));
    assert!(!output.code.contains("--"));
    Ok(())
}

#[test]
#[serial]
fn reads_commonjs_exports_when_compiled_jsx_precedes_assignment() -> Result<(), Box<dyn Error>> {
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "tokens.mdown".to_string(),
            code: "function view() { return <div/>; } exports.PRIMARY = 'blue';".to_string(),
            source_type: Some(ExtractSourceType::CompiledMdx),
        }))
    };
    let output = extract_with_source_type(
        "page.tsx",
        "import {Box} from '@devup-ui/react'; import {PRIMARY} from './tokens'; export const view = <Box color={PRIMARY}/>;",
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
    )?;
    assert!(!output.code.contains("--"));
    assert_eq!(output.dependencies, ["tokens.mdown"]);
    Ok(())
}

#[test]
#[serial]
fn generates_class_names_when_explicit_mode_keeps_custom_filename() -> Result<(), Box<dyn Error>> {
    let names = FxHashSet::from_iter(["card".to_string()]);
    let classes = extract_class_map_from_code(
        "page.mdown",
        "import {css} from '@devup-ui/react'; export const card = css({color:'blue'});",
        &ExtractOption::default(),
        &names,
        Some(ExtractSourceType::CompiledMdx),
    )?;
    assert!(classes.contains_key("card"));
    Ok(())
}

#[rstest::rstest]
#[case("page.md")]
#[case("page.mdx")]
#[case("page.mdown")]
#[case("page.ts")]
#[serial]
fn extracts_namespace_computation_when_compiled_jsx_precedes_use(
    #[case] filename: &str,
    #[values(true, false)] source_map: bool,
) -> Result<(), Box<dyn Error>> {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let code = r#"import * as E from "@emotion/css";
import {css as dc, Box} from "@devup-ui/react";
export const preceding = <Box bg="tomato"/>;
const key="css", n=2; export const cls=E[key]({margin:n*4,opacity:n/4});
export const control = dc({p:3});"#;
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let output = extract_with_source_type(
        filename,
        code,
        option,
        source_map,
        None,
        Some(ExtractSourceType::CompiledMdx),
    )?;
    // Then
    for (property, value) in [
        ("margin", "8px"),
        ("padding", "12px"),
        ("background", "tomato"),
    ] {
        assert!(output.styles.iter().any(|style| matches!(
            style, ExtractStyleValue::Static(style) if style.property == property && style.value == value
        )), "missing {property}:{value}");
    }
    assert!(
        output.styles.iter().any(|style| matches!(
            style, ExtractStyleValue::Static(style) if style.property == "opacity"
                && style.value.parse::<f64>().is_ok_and(|value| (value - 0.5).abs() < f64::EPSILON)
        )),
        "missing opacity:0.5"
    );
    let runtime: String = output.code.split_whitespace().collect();
    assert!(
        !runtime.contains("E[") && !runtime.contains("E."),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest::rstest]
#[case("page.md")]
#[case("page.mdx")]
#[case("page.mdown")]
#[case("page.ts")]
#[serial]
fn rejects_acquired_styling_when_compiled_jsx_precedes_use(
    #[case] filename: &str,
    #[values(true, false)] source_map: bool,
) {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let code = concat!(
        "const E = require(\"@emotion/css\");\nexport const preceding=<div/>;\nexport const a=E.css({",
        "padding:8});",
    );
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let error = match extract_with_source_type(
        filename,
        code,
        option,
        source_map,
        None,
        Some(ExtractSourceType::CompiledMdx),
    ) {
        Ok(output) => panic!("acquired styling unexpectedly compiled: {output:?}"),
        Err(error) => error.to_string(),
    };
    // Then
    assert!(error.starts_with(&format!("{filename}:3:16:")), "{error}");
    assert!(error.contains("cannot use `E.css`"), "{error}");
    assert!(error.contains("styling APIs read through require(), import-equals or dynamic import cannot be compiled"), "{error}");
}
