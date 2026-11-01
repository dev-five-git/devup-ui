use serial_test::serial;

use crate::{ExtractOption, ResolvedModule};

#[rstest::rstest]
#[case("import styles from './styles.module.css';")]
#[case("import {card} from './styles.module.css';")]
#[case("import * as styles from './styles.module.css';")]
#[serial]
fn css_retention_keeps_unused_root_value_imports_when_source_is_javascript(
    #[case] import: &str,
    #[values("js", "jsx", "mjs")] extension: &str,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let source = format!(
        "import './before.css';{import}import './after.css';export const value=['red'].join('');"
    );
    let filename = format!("/src/card.css.{extension}");
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./styles.module.css").then(|| ResolvedModule {
            path: "/src/styles.module.css".to_string(),
            code: super::super::retained_css::CSS.to_string(),
        })
    };
    // When
    let (_, imports) = crate::vanilla_extract::execute_stylesheet(
        &source,
        &filename,
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(
        imports.kept_imports,
        ["./before.css", "./styles.module.css", "./after.css"]
    );
    Ok(())
}

#[rstest::rstest]
#[case("const styles=require('./styles.module.css');")]
#[serial]
fn css_retention_keeps_unused_value_loads_when_helper_is_commonjs(
    #[case] declaration: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    css::file_map::reset_file_map();
    let helper = format!("{declaration}exports.color='red';");
    let resolver = move |specifier: &str, importer: &str| match (specifier, importer) {
        ("./helper", "/src/card.css.ts") => Some(ResolvedModule {
            path: "/src/helper.cjs".to_string(),
            code: helper.clone(),
        }),
        ("./styles.module.css", "/src/helper.cjs") => Some(ResolvedModule {
            path: "/src/styles.module.css".to_string(),
            code: super::super::retained_css::CSS.to_string(),
        }),
        _ => None,
    };
    // When
    let output = crate::extract_with_modules(
        "/src/card.css.ts",
        super::super::retained_css::ENTRY,
        ExtractOption::default(),
        false,
        &resolver,
    )?;
    // Then
    assert!(
        output.code.contains("import \"./styles.module.css\";"),
        "{}",
        output.code
    );
    assert_eq!(output.dependencies, ["/src/helper.cjs"]);
    Ok(())
}

#[rstest::rstest]
#[case("function neverCalled(){return styles.card;}", true)]
#[case("type Styles = typeof styles;", false)]
#[serial]
fn css_retention_distinguishes_value_and_type_references_when_root_is_typescript(
    #[case] reference: &str,
    #[case] retained: bool,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let source = format!(
        "import styles from './styles.module.css';{reference}export const value=['red'].join('');"
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./styles.module.css").then(|| ResolvedModule {
            path: "/src/styles.module.css".to_string(),
            code: super::super::retained_css::CSS.to_string(),
        })
    };
    // When
    let (_, imports) = crate::vanilla_extract::execute_stylesheet(
        &source,
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    let expected = if retained {
        vec!["./styles.module.css"]
    } else {
        vec![]
    };
    assert_eq!(imports.kept_imports, expected);
    Ok(())
}

#[test]
#[serial]
fn css_retention_keeps_unused_value_loads_when_root_is_commonjs() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./styles.module.css").then(|| ResolvedModule {
            path: "/src/styles.module.css".to_string(),
            code: super::super::retained_css::CSS.to_string(),
        })
    };
    // When
    let (_, imports) = crate::vanilla_extract::execute_stylesheet(
        "const styles=require('./styles.module.css');const value=['red'].join('');",
        "/src/card.css.cjs",
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(imports.kept_imports, ["./styles.module.css"]);
    Ok(())
}

#[test]
#[serial]
fn css_retention_rejects_actual_read_when_helper_is_commonjs() {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| match (specifier, importer) {
        ("./helper", "/src/card.css.ts") => Some(ResolvedModule {
            path: "/src/helper.cjs".to_string(),
            code: "const styles=require('./styles.module.css');\nexports.color=styles.card;"
                .to_string(),
        }),
        ("./styles.module.css", "/src/helper.cjs") => Some(ResolvedModule {
            path: "/src/styles.module.css".to_string(),
            code: super::super::retained_css::CSS.to_string(),
        }),
        _ => None,
    };
    // When
    let error = crate::extract_with_modules(
        "/src/card.css.ts",
        super::super::retained_css::ENTRY,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();
    // Then
    assert!(error.starts_with("/src/helper.cjs:2:15:"), "{error}");
    assert!(
        error.contains("export 'card'") && error.contains("/src/styles.module.css"),
        "{error}"
    );
}
