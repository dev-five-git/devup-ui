use serial_test::serial;

use crate::{ExtractOption, ResolvedModule};

pub(super) const ENTRY: &str = "import {style} from '@devup-ui/react';import {color} from './helper';export const card=style({color});";
pub(super) const CSS: &str = ".card { color: red; }";

#[rstest::rstest]
#[case(
    "/src/card.css.ts",
    "/src/helper.ts",
    "./global.css",
    "/src/global.css",
    "./global.css"
)]
#[case(
    "/src/pages/card.css.ts",
    "/src/helpers/helper.ts",
    "./global.css",
    "/src/helpers/global.css",
    "../helpers/global.css"
)]
#[case(
    "/src/card.css.ts",
    "/src/helper.ts",
    "theme/package.css",
    "/node_modules/theme/dist/package.css",
    "../node_modules/theme/dist/package.css"
)]
#[case(
    "/src/card.css.ts",
    "/src/helper.ts",
    "./global.CSS?inline",
    "/src/global.CSS?inline",
    "./global.CSS?inline"
)]
#[case(
    "C:/app/src/card.css.ts",
    "C:/app/helpers/helper.ts",
    "./global.css",
    "C:\\app\\helpers\\global.css",
    "../helpers/global.css"
)]
#[serial]
fn css_retention_emits_portable_specifiers_when_helper_loads_css(
    #[case] root: &'static str,
    #[case] helper: &'static str,
    #[case] imported: &'static str,
    #[case] target: &'static str,
    #[case] emitted: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: the resolver discriminates importer directories and package exports.
    css::file_map::reset_file_map();
    let helper_code = format!("import {imported:?};export const color='red';");
    let resolver = move |specifier: &str, importer: &str| {
        let (path, code) = if (specifier, importer) == ("./helper", root) {
            (helper, helper_code.as_str())
        } else if (specifier, importer) == (imported, helper)
            || (specifier, importer) == (emitted, root)
        {
            (target, CSS)
        } else {
            return None;
        };
        Some(ResolvedModule {
            path: path.to_string(),
            code: code.to_string(),
        })
    };
    // When
    let output =
        crate::extract_with_modules(root, ENTRY, ExtractOption::default(), false, &resolver)?;
    // Then: the emitted load resolves to the helper's actual CSS, not root/global.css.
    let allocator = oxc_allocator::Allocator::default();
    let parsed =
        oxc_parser::Parser::new(&allocator, &output.code, oxc_span::SourceType::mjs()).parse();
    let loads: Vec<_> = parsed
        .program
        .body
        .iter()
        .filter_map(|statement| match statement {
            oxc_ast::ast::Statement::ImportDeclaration(import)
                if !crate::package_specifier::is_package(
                    import.source.value.as_str(),
                    "@devup-ui/react",
                ) =>
            {
                Some(import.source.value.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(loads, [emitted]);
    assert_eq!(
        resolver(loads[0], root).map(|module| module.path),
        Some(target.to_string())
    );
    assert_eq!(output.dependencies, [helper]);
    Ok(())
}

#[rstest::rstest]
#[case("import './root.css';import {color} from './helper';import './after.css';", vec!["./root.css", "./shared.css?inline", "./child.css", "./helper.css", "./after.css"])]
#[case("import './shared-alias.css';import {color} from './helper';import './after.css';", vec!["./shared-alias.css", "./child.css", "./helper.css", "./after.css"])]
#[case("import {color} from './helper';import './shared-alias.css';import './after.css';", vec!["./shared.css?inline", "./child.css", "./helper.css", "./after.css"])]
#[case("import './shared-alias.css';import './shared.css?inline';import {color} from './helper';", vec!["./shared-alias.css", "./child.css", "./helper.css"])]
#[serial]
fn css_retention_orders_and_deduplicates_when_root_and_helpers_interleave(
    #[case] imports: &str,
    #[case] expected: Vec<&str>,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| {
        let (path, code) = match (specifier, importer) {
            ("./helper", "/src/card.css.ts") => (
                "/src/helper.ts",
                "import './child';import './helper.css';import './child';export const color='red';",
            ),
            ("./child", "/src/helper.ts") => (
                "/src/child.ts",
                "import './shared.css?inline';import './child.css';",
            ),
            ("./shared.css?inline", "/src/child.ts" | "/src/card.css.ts")
            | ("./shared-alias.css", "/src/card.css.ts") => ("/src/shared.css", CSS),
            ("./root.css", "/src/card.css.ts") => ("/src/root.css", CSS),
            ("./after.css", "/src/card.css.ts") => ("/src/after.css", CSS),
            ("./child.css", "/src/child.ts" | "/src/card.css.ts") => ("/src/child.css", CSS),
            ("./helper.css", "/src/helper.ts" | "/src/card.css.ts") => ("/src/helper.css", CSS),
            _ => return None,
        };
        Some(ResolvedModule {
            path: path.to_string(),
            code: code.to_string(),
        })
    };
    let source = format!("{imports}export const value=color;");
    // When
    let (_, output) = crate::vanilla_extract::execute_stylesheet(
        &source,
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(output.kept_imports, expected);
    for specifier in &output.kept_imports {
        assert!(
            resolver(specifier, "/src/card.css.ts").is_some(),
            "{specifier}"
        );
    }
    assert_eq!(
        output.dependencies.into_iter().collect::<Vec<_>>(),
        ["/src/child.ts", "/src/helper.ts"]
    );
    Ok(())
}

#[test]
#[serial]
fn css_retention_locates_missing_file_when_helper_imports_it() {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| {
        ((specifier, importer) == ("./helper", "/src/card.css.ts")).then(|| ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: "const label: string='한글😀';\nimport './missing.css';export const color='red';"
                .to_string(),
        })
    };
    // When
    let error = crate::extract_with_modules(
        "/src/card.css.ts",
        ENTRY,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();
    // Then
    assert!(error.starts_with("/src/helper.ts:2:8:"), "{error}");
    assert!(
        error.contains("./missing.css") && error.contains("cannot be found"),
        "{error}"
    );
    assert!(
        error.contains("stylesheet loads CSS through this helper"),
        "{error}"
    );
    assert!(
        error.contains("Fix: correct the path or import CSS from a component module"),
        "{error}"
    );
}

#[test]
#[serial]
fn css_retention_locates_drive_error_when_target_is_on_another_drive() {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| match (specifier, importer) {
        ("./helper", "C:/src/card.css.ts") => Some(ResolvedModule {
            path: "C:/src/helper.ts".to_string(),
            code: "import './global.css';export const color='red';".to_string(),
        }),
        ("./global.css", "C:/src/helper.ts") => Some(ResolvedModule {
            path: "D:/styles/global.css".to_string(),
            code: CSS.to_string(),
        }),
        _ => None,
    };
    // When
    let error = crate::extract_with_modules(
        "C:/src/card.css.ts",
        ENTRY,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();
    // Then
    assert!(error.starts_with("C:/src/helper.ts:1:8:"), "{error}");
    assert!(
        error.contains("different Windows drives") && error.contains("D:/styles/global.css"),
        "{error}"
    );
    assert!(error.contains("Fix:"), "{error}");
}

#[test]
#[serial]
fn css_retention_preserves_exemptions_when_helper_has_generated_and_api_loads() -> Result<(), String>
{
    // Given
    css::file_map::reset_file_map();
    let option = ExtractOption {
        css_dir: "df/styles".to_string(),
        ..ExtractOption::default()
    };
    let resolver = |specifier: &str, importer: &str| {
        assert_eq!((specifier, importer), ("./helper", "/src/card.css.ts"));
        Some(ResolvedModule { path: "/src/helper.ts".to_string(), code: "import '@devup-ui/react/devup-ui.css';import '@devup-ui/react/compat';import 'df/styles/devup-ui.css';export const color='red';".to_string() })
    };
    // When
    let (_, imports) = crate::vanilla_extract::execute_stylesheet(
        "import {color} from './helper';export const value=color;",
        "/src/card.css.ts",
        &option,
        Some(&resolver),
    )?;
    // Then
    assert_eq!(imports.kept_imports, Vec::<String>::new());
    assert_eq!(
        imports.dependencies.into_iter().collect::<Vec<_>>(),
        ["/src/helper.ts"]
    );
    Ok(())
}
