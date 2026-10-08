use serial_test::serial;

use crate::{ExtractOption, ResolvedModule};

const ENTRY: &str = "import {style} from '@devup-ui/react'; import {color} from './helper'; export const card=style({color});";

#[rstest::rstest]
#[case(
    "throw new Error('side-effect boom');",
    "/src/effect.ts:1:7",
    "Error: side-effect boom"
)]
#[case(
    "const label: string = '한글😀';\nfunction fail(): void {\n  throw new Error('side-effect boom');\n}\nfail();",
    "/src/effect.ts:3:9",
    "Error: side-effect boom"
)]
#[case(
    "try{Date.now()}catch{}",
    "/src/effect.ts:1:5",
    "ReferenceError: `Date`"
)]
#[case(
    "const label: string = '한글😀';\ntry {\n  Date.now();\n} catch {}",
    "/src/effect.ts:3:3",
    "ReferenceError: `Date`"
)]
#[serial]
fn side_effect_dependency_failures_keep_original_locations(
    #[case] effect: &'static str,
    #[case] location: &str,
    #[case] cause: &str,
) {
    // Given
    css::file_map::reset_file_map();
    let resolver = move |specifier: &str, importer: &str| match (specifier, importer) {
        ("./helper", "/src/card.css.ts") => Some(ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: "import './effect';export const color='red';".to_string(),
        }),
        ("./effect", "/src/helper.ts") => Some(ResolvedModule {
            path: "/src/effect.ts".to_string(),
            code: effect.to_string(),
        }),
        _ => None,
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
    assert!(
        error.starts_with(&format!("{location}: JS execution error: {cause}")),
        "{error}"
    );
    assert!(error.contains(". Fix: "), "{error}");
    assert!(error.contains(&format!("({location})")), "{error}");
    assert!(!error.contains("devup-ui-stylesheet"), "{error}");
}

#[test]
#[serial]
fn side_effect_dependency_missing_import_is_located_in_helper() {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| {
        (specifier == "./helper" && importer == "/src/card.css.ts").then(|| ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: "const label: string = '한글😀';\nimport './missing';\nexport const color='red';"
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
    assert!(
        error.starts_with(
            "/src/helper.ts:2:8: Cannot resolve './missing' from '/src/helper.ts'. Fix: "
        ),
        "{error}"
    );
}

#[test]
#[serial]
fn side_effect_dependency_pure_module_is_tracked_in_output()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| match (specifier, importer) {
        ("./helper", "/src/card.css.ts") => Some(ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: "import './effect';export const color='red';".to_string(),
        }),
        ("./effect", "/src/helper.ts") => Some(ResolvedModule {
            path: "/src/effect.ts".to_string(),
            code: "export const unused: string = 'pure';".to_string(),
        }),
        _ => None,
    };
    // When
    let output = crate::extract_with_modules(
        "/src/card.css.ts",
        ENTRY,
        ExtractOption::default(),
        false,
        &resolver,
    )?;
    // Then
    assert_eq!(output.dependencies, ["/src/effect.ts", "/src/helper.ts"]);
    assert!(output.styles.iter().any(|style| matches!(style,
        crate::ExtractStyleValue::Static(value) if value.property() == "color" && value.value() == "red"
    )));
    Ok(())
}

#[test]
#[serial]
fn side_effect_dependency_duplicate_diamond_cycle_executes_once_in_order() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| {
        let (path, code) = match (specifier, importer) {
            ("./helper", "/src/card.css.ts") => (
                "/src/helper.ts",
                "import {events} from './init';import './left';import './left';import './right';events.push('helper');export const color=events.join(',');",
            ),
            ("./init", "/src/helper.ts" | "/src/left.ts" | "/src/right.ts" | "/src/shared.ts") => {
                ("/src/init.ts", "export const events=['init'];")
            }
            ("./left", "/src/helper.ts" | "/src/shared.ts") => (
                "/src/left.ts",
                "import {events} from './init';import './shared';events.push('left');",
            ),
            ("./right", "/src/helper.ts") => (
                "/src/right.ts",
                "import {events} from './init';import './shared-alias';events.push('right');",
            ),
            ("./shared", "/src/left.ts") | ("./shared-alias", "/src/right.ts") => (
                "/src/shared.ts",
                "import {events} from './init';import './left';events.push('shared');",
            ),
            _ => return None,
        };
        Some(ResolvedModule {
            path: path.to_string(),
            code: code.to_string(),
        })
    };
    // When
    let (collected, imports) = crate::vanilla_extract::execute_stylesheet(
        "import {color} from './helper';export const order=color;",
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [(
            "order".to_string(),
            "\"init,shared,left,right,helper\"".to_string()
        )]
    );
    assert_eq!(
        imports.dependencies.into_iter().collect::<Vec<_>>(),
        [
            "/src/helper.ts",
            "/src/init.ts",
            "/src/left.ts",
            "/src/right.ts",
            "/src/shared.ts"
        ]
    );
    Ok(())
}

#[rstest::rstest]
#[case("./root-effect")]
#[case("external-package")]
#[case("./global.css")]
#[case("@devup-ui/react-values")]
#[serial]
fn side_effect_dependency_entry_imports_stay_kept(#[case] specifier: &str) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let source = format!("import '{specifier}';export const color='red';");
    let resolver = |_: &str, _: &str| -> Option<ResolvedModule> {
        panic!("entry bare import must not be loaded")
    };
    // When
    let (collected, imports) = crate::vanilla_extract::execute_stylesheet(
        &source,
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [("color".to_string(), "\"red\"".to_string())]
    );
    assert_eq!(imports.kept_imports, [specifier]);
    assert_eq!(imports.dependencies.len(), 0);
    Ok(())
}

#[test]
#[serial]
fn side_effect_dependency_helper_css_and_api_subpaths_are_not_evaluated() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| {
        match (specifier, importer) {
            ("./helper", "/src/card.css.ts") => Some(ResolvedModule {
                path: "/src/helper.ts".to_string(),
                code: "import './global.css';import '@devup-ui/react';import '@devup-ui/react/compat';import '@devup-ui/react/styles';export const color='red';".to_string(),
            }),
            ("./global.css", "/src/helper.ts") => Some(ResolvedModule {
                path: "/src/global.css".to_string(),
                code: ".card { color: red; }".to_string(),
            }),
            _ => panic!("API subpaths must not be resolved: {specifier} from {importer}"),
    }
    };
    // When
    let (collected, imports) = crate::vanilla_extract::execute_stylesheet(
        "import {color} from './helper';export const colorValue=color;",
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [("colorValue".to_string(), "\"red\"".to_string())]
    );
    assert_eq!(imports.dependencies.len(), 1);
    assert!(imports.dependencies.contains("/src/helper.ts"));
    assert_eq!(imports.kept_imports, ["./global.css"]);
    Ok(())
}

#[test]
#[serial]
fn side_effect_dependency_external_sibling_package_executes() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| {
        match (specifier, importer) {
        ("./helper", "/src/card.css.ts") => Some(ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: "import {state} from './state';import '@devup-ui/react-values';export const color=state.color;"
                .to_string(),
        }),
        ("@devup-ui/react-values", "/src/helper.ts") => Some(ResolvedModule {
            path: "/node_modules/@devup-ui/react-values/index.js".to_string(),
            code: "import {state} from './state';state.color='blue';".to_string(),
        }),
        ("./state", "/src/helper.ts" | "/node_modules/@devup-ui/react-values/index.js") => Some(ResolvedModule {
            path: "/src/state.ts".to_string(),
            code: "export const state={color:'red'};".to_string(),
        }),
        _ => None,
    }
    };
    // When
    let (collected, imports) = crate::vanilla_extract::execute_stylesheet(
        "import {color} from './helper';export const colorValue=color;",
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
    )?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [("colorValue".to_string(), "\"blue\"".to_string())]
    );
    assert_eq!(
        imports.dependencies.into_iter().collect::<Vec<_>>(),
        [
            "/node_modules/@devup-ui/react-values/index.js",
            "/src/helper.ts",
            "/src/state.ts"
        ]
    );
    Ok(())
}
