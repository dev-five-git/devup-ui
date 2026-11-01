use crate::{
    ExtractOption, ExtractStyleValue, ImportAlias, ResolvedModule, extract, extract_with_modules,
};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("const value = ;")]
#[case("let value = 1; let value = 2;")]
#[serial]
fn invalid_commonjs_source_is_rejected_when_module_input_is_supported(
    #[case] body: &str,
) -> Result<(), String> {
    // Given
    let source = format!("import {{ styled }} from '@devup-ui/react'; {body}");
    // When
    let error = extract("invalid.cjs", &source, ExtractOption::default())
        .err()
        .ok_or("invalid source must fail")?
        .to_string();
    // Then
    assert!(error.starts_with("invalid.cjs:1:"), "{error}");
    assert!(error.contains("Cannot parse source:"), "{error}");
    assert!(
        !error.contains("Cannot use import statement outside a module"),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case("__at")]
#[case("__devup_read_site_authored")]
#[case("__devup_operation_site_authored")]
#[serial]
fn authored_frames_survive_when_names_resemble_instrumentation(
    #[case] name: &str,
) -> Result<(), String> {
    // Given
    let source = format!("function {name}() {{ throw new Error('authored'); }}\n{name}();");
    // When
    let error = crate::vanilla_extract::execute_stylesheet(
        &source,
        "frames.css.ts",
        &ExtractOption::default(),
        None,
    )
    .err()
    .ok_or("authored function must throw")?;
    // Then
    assert!(
        error.contains(&format!("    at {name} (frames.css.ts:1:")),
        "{error}"
    );
    assert_eq!(error.matches("(native)").count(), 0, "{error}");
    Ok(())
}

#[test]
#[serial]
fn authored_native_activation_survives_when_it_calls_the_instrumentation_api() -> Result<(), String>
{
    // Given
    let source = "__vanilla_extract__.__at('authored', 'call', () => { throw new Error('authored native'); });";
    // When
    let error = crate::vanilla_extract::execute_stylesheet(
        source,
        "native.css.ts",
        &ExtractOption::default(),
        None,
    )
    .err()
    .ok_or("authored callback must throw")?;
    // Then
    assert_eq!(error.matches("    at __at (native)").count(), 1, "{error}");
    Ok(())
}

#[rstest]
#[case("__devup_read_site_123__")]
#[case("__devup_operation_site_456__")]
#[serial]
fn authored_hash_names_survive_when_they_match_raw_stack_helper_conventions(
    #[case] name: &str,
) -> Result<(), String> {
    // Given
    let source = format!("function {name}() {{ throw new Error('authored hash'); }}\n{name}();");
    // When
    let error = crate::vanilla_extract::execute_stylesheet(
        &source,
        "hash.css.ts",
        &ExtractOption::default(),
        None,
    )
    .err()
    .ok_or("authored hash function must throw")?;
    // Then
    assert!(
        error.contains(&format!("    at {name} (hash.css.ts:1:")),
        "{error}"
    );
    assert_eq!(error.matches("(native)").count(), 0, "{error}");
    Ok(())
}

#[rstest]
#[case("")]
#[case("const forceRun = (() => 1)();")]
#[serial]
fn global_paths_survive_when_plain_or_forced_to_run(#[case] prefix: &str) -> Result<(), String> {
    // Given
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\n{prefix}\nglobalCss({{'@media print':{{body:{{boxAlign:'center'}}}}}});"
    );
    let column = source
        .rsplit('\n')
        .next()
        .and_then(|line| line.find("boxAlign"))
        .ok_or("fixture must contain the authored key")?
        + 1;
    // When
    let error = extract("paths.css.ts", &source, ExtractOption::default())
        .err()
        .ok_or("invalid global declaration must fail")?
        .to_string();
    // Then
    assert!(
        error.starts_with(&format!("paths.css.ts:3:{column}:")),
        "{error}"
    );
    assert!(
        error.contains("@media print -> body -> boxAlign"),
        "{error}"
    );
    Ok(())
}

#[test]
#[serial]
fn helper_style_keeps_numeric_units_when_an_ordinary_module_is_evaluated() -> Result<(), String> {
    // Given
    reset_class_map();
    reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./helper").then(|| ResolvedModule {
        path: "helper.ts".to_string(),
        code: "import {style} from '@vanilla-extract/css'; export const make=()=>style({padding:2,opacity:0.5});".to_string(),
    })
    };
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    // When
    let output = extract_with_modules("helper-entry.css.ts", "import {style} from '@vanilla-extract/css'; import {make} from './helper'; export const c=style(make());", option, false, &resolver).map_err(|error| error.to_string())?;
    // Then
    let declarations: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property.as_str(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert!(
        declarations.contains(&("padding", "2px")),
        "{declarations:?}"
    );
    assert!(
        declarations.contains(&("opacity", ".5")),
        "{declarations:?}"
    );
    Ok(())
}
