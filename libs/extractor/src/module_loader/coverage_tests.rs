use rstest::rstest;

use super::{ModuleLoader, Unit, module_script};
use crate::ExtractOption;

#[test]
fn loaded_module_bodies_execute_before_the_importing_entry() -> Result<(), String> {
    // Given
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./tokens").then(|| crate::ResolvedModule {
            path: "tokens.ts".to_string(),
            code: "export const color = 'red';".to_string(),
        })
    };
    let option = ExtractOption::default();
    let mut loader = ModuleLoader::new(Some(&resolver), &option);
    let code = "import { color } from './tokens'; const result = color;";
    let unit = Unit::written("entry.ts", code, code, &[])?;
    let entry = module_script(&unit, &mut loader, true)?;
    let script = loader.script(&entry);
    let mut context = boa_engine::Context::default();
    // When
    let value = context
        .eval(boa_engine::Source::from_bytes(&format!(
            "{}\nresult",
            script.text
        )))
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(
        value
            .to_string(&mut context)
            .map_err(|error| error.to_string())?
            .to_std_string_escaped(),
        "red"
    );
    assert_eq!(
        loader.dependencies.into_iter().collect::<Vec<_>>(),
        ["tokens.ts"]
    );
    Ok(())
}

#[rstest]
#[case("return 1;", "SyntaxError")]
#[case("const a = 1; const a = 2;", "SyntaxError")]
fn module_script_rejects_generated_parse_and_semantic_errors_with_file_fallback(
    #[case] code: &str,
    #[case] category: &str,
) -> Result<(), String> {
    // Given
    let unit = Unit::generated("invalid.ts", code);
    let option = ExtractOption::default();
    let mut loader = ModuleLoader::new(None, &option);
    // When
    let error = module_script(&unit, &mut loader, true)
        .err()
        .ok_or("invalid generated module accepted")?;
    // Then
    assert!(
        error.starts_with(&format!("invalid.ts:1:1: JS execution error: {category}:")),
        "{error}"
    );
    Ok(())
}

#[path = "loader_script_coverage_tests.rs"]
mod loader_script_coverage_tests;
