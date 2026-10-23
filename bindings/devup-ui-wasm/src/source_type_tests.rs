use super::*;
use serial_test::serial;

#[test]
#[serial]
fn source_type_extracts_compiled_mdx_when_native_variants_use_real_filename() -> Result<(), String>
{
    reset_build_state_internal();
    let code = "import {Box} from '@devup-ui/react'; export const view = <Box color='blue'/>;";
    let mode = Some(extractor::ExtractSourceType::CompiledMdx);
    let mapped = code_extract_internal(
        "page.mdown",
        code,
        "@devup-ui/react",
        "df".to_string(),
        true,
        false,
        false,
        HashMap::new(),
        mode,
    )?;
    assert!(mapped.map().is_some_and(|map| map.contains("page.mdown")));
    let unmapped = code_extract_without_source_map_internal(
        "page.mdown",
        code,
        "@devup-ui/react",
        "df".to_string(),
        true,
        false,
        false,
        HashMap::new(),
        mode,
    )?;
    assert_eq!(unmapped.map(), None);
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "tokens.mdown".to_string(),
            code: "export function view() {return <div/>;} export const PRIMARY = 'blue';"
                .to_string(),
            source_type: mode,
        }))
    };
    let imported = code_extract_with_modules_internal(
        "page.mdown",
        "import {Box} from '@devup-ui/react'; import {PRIMARY} from './tokens'; export const view = <Box color={PRIMARY}/>;",
        "@devup-ui/react",
        "df".to_string(),
        true,
        false,
        false,
        HashMap::new(),
        mode,
        &resolver,
    )?;
    assert_eq!(imported.dependencies(), ["tokens.mdown"]);
    assert!(!with_style_sheet(|sheet| sheet.create_css(None, false)).contains("var(--"));
    reset_build_state_internal();
    Ok(())
}

#[test]
#[serial]
fn source_type_preserves_sheet_when_unknown_child_validation_fails() -> Result<(), String> {
    reset_build_state_internal();
    code_extract_internal(
        "page.tsx",
        "import {Box} from '@devup-ui/react'; export const view = <Box color='blue'/>;",
        "@devup-ui/react",
        "df".to_string(),
        true,
        false,
        false,
        HashMap::new(),
        None,
    )?;
    let before = export_sheet_internal()?;
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "tokens.custom".to_string(),
            code: "export const PRIMARY = 'red';".to_string(),
            source_type: None,
        }))
    };
    let result = code_extract_with_modules_internal(
        "page.tsx",
        "import {Box} from '@devup-ui/react'; import {PRIMARY} from './tokens'; export const view = <Box color={PRIMARY}/>;",
        "@devup-ui/react",
        "df".to_string(),
        true,
        false,
        false,
        HashMap::new(),
        None,
        &resolver,
    );
    assert!(result.is_err_and(|error| error.starts_with("tokens.custom:1:1:")));
    assert_eq!(export_sheet_internal()?, before);
    reset_build_state_internal();
    Ok(())
}
