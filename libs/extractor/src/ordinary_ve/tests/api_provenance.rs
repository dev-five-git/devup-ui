use oxc_ast::ast::{
    Argument, Declaration, Expression, ImportDeclarationSpecifier, ObjectPropertyKind, Statement,
};
use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::{assert_consumed, has_static};

#[rstest]
#[case((false, "export {style as make} from '@vanilla-extract/css';", "import {make} from './api';", "make"))]
#[case((false, "export * from './terminal';", "import {style as make} from './api';", "make"))]
#[case((false, "export * as native from './terminal';", "import {native} from './api';", "native.style"))]
#[case((false, "export {style as default} from '@vanilla-extract/css';", "import make from './api';", "make"))]
#[case((false, concat!("import * as ve from '@vanilla-extract/css';const {", "style:make}=ve;export {make};"), "import {make} from './api';", "make"))]
#[case((false, "import {style} from '@vanilla-extract/css';const make=style;export default make;", "import make from './api';", "make"))]
#[case((false, "import * as ve from '@vanilla-extract/css';const native=ve;export {native};", "import {native} from './api';", "native.style"))]
#[case((true, "export {style as make} from '@vanilla-extract/css';", "import {make} from './api';", "make"))]
#[case((true, "export * from './terminal';", "import {style as make} from './api';", "make"))]
#[case((true, "export * as native from './terminal';", "import {native} from './api';", "native.style"))]
#[case((true, "export {style as default} from '@vanilla-extract/css';", "import make from './api';", "make"))]
#[serial]
fn actual_native_terminals_compile_when_barrels_feed_entry_or_imported_producer(
    #[case] fixture: (bool, &str, &str, &str),
) -> TestResult {
    // Given
    reset();
    let (imported, export, import, call) = fixture;
    let barrel = format!(
        "{export}export const browser=window.document;throw new Error('api barrel runtime only');"
    );
    let producer = format!(
        "{import}export const base={call}({{color:'blue',padding:8}});const browser=window.document;throw new Error('api producer runtime only');"
    );
    let source = if imported {
        "import {css} from '@devup-ui/react';import {base} from './producer';export const box=css(base,{color:'green'});const browser=window.document;".to_string()
    } else {
        producer.clone()
    };
    let graph = [
        ("./api", "/api.ts", barrel.as_str()),
        (
            "./terminal",
            "/terminal.ts",
            "export {style} from '@vanilla-extract/css';",
        ),
        ("./producer", "/api-producer.ts", producer.as_str()),
    ];
    // When
    let output = run("/api-entry.ts", &source, &graph)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert!(has_static(
        &output,
        "color",
        if imported { "green" } else { "blue" }
    ));
    if imported {
        assert!(!has_static(&output, "color", "blue"));
    }
    assert!(
        !output.code.contains(&format!("{call}(")),
        "{}",
        output.code
    );
    assert!(output.code.contains("window.document"), "{}", output.code);
    assert_import(&output, if imported { "./producer" } else { "./api" });
    assert!(output.dependencies.iter().any(|path| path == "/api.ts"));
    if imported {
        assert!(
            output
                .dependencies
                .iter()
                .any(|path| path == "/api-producer.ts")
        );
    }
    if export.contains("./terminal") {
        assert!(
            output
                .dependencies
                .iter()
                .any(|path| path == "/terminal.ts")
        );
    }
    Ok(())
}

#[rstest]
#[case("import * as ve from '@vanilla-extract/css';", "ve['style']")]
#[case(
    concat!("import * as ve from '@vanilla-extract/css';const {", "style:make}=ve;"),
    "make"
)]
#[case(
    concat!("import * as ve from '@vanilla-extract/css';const native=ve;const {", "style:make}=native;"),
    "make"
)]
#[serial]
fn original_direct_provenance_is_consumed_when_bindings_are_immutable_aliases(
    #[case] binding: &str,
    #[case] call: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "{binding}export const box={call}({{color:'blue',padding:8}});const browser=window.document;"
    );
    // When
    let output = run("/direct.ts", &source, &[])?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(
        !output.code.contains(&format!("{call}(")),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case(
    "export * from './left';export * from './right';",
    "export {style} from '@vanilla-extract/css';",
    "8px"
)]
#[case(
    "export * from './left';export {style} from './own';",
    "export const style=value=>({margin:'13px'});",
    "13px"
)]
#[serial]
fn export_identity_follows_esm_when_duplicate_terminals_or_own_shadow_exist(
    #[case] barrel: &str,
    #[case] own: &str,
    #[case] expected: &str,
) -> TestResult {
    // Given
    reset();
    let source = "import {style} from './api';import {createTheme} from '@vanilla-extract/css';import {css} from '@devup-ui/react';const [theme,vars]=createTheme({space:'8px'});const base=style({margin:8});export const box=css(base);const browser=window.document;";
    let graph = [
        ("./api", "/identity.ts", barrel),
        (
            "./left",
            "/left.ts",
            "export {style} from '@vanilla-extract/css';",
        ),
        (
            "./right",
            "/right.ts",
            "export {style} from '@vanilla-extract/css';",
        ),
        ("./own", "/own.ts", own),
    ];
    // When
    let output = run("/identity-entry.ts", source, &graph)?;
    // Then
    assert!(has_static(&output, "margin", expected));
    assert!(!has_static(&output, "margin", "32px"));
    Ok(())
}

#[test]
#[serial]
fn competing_star_origins_error_when_a_requested_api_has_no_unique_binding() -> TestResult {
    // Given
    reset();
    let source = "import {style} from './api';\nexport const box=style({color:'blue'});const browser=window.document;";
    let place = crate::locate(
        "/ambiguous.ts",
        source,
        source.find("style}").ok_or("fixture import missing")?,
    );
    let graph = [
        (
            "./api",
            "/ambiguous-api.ts",
            "export * from './left';export * from './right';",
        ),
        (
            "./left",
            "/left.ts",
            "export {style} from '@vanilla-extract/css';",
        ),
        ("./right", "/right.ts", "export const style=value=>value;"),
    ];
    // When
    let result = run("/ambiguous.ts", source, &graph);
    // Then
    located_failure(result, &place, "ambiguous");
    Ok(())
}

#[rstest]
#[case(
    "import {style} from '@vanilla-extract/css';import * as ve from '@vanilla-extract/css';import {css} from '@devup-ui/react';export function render(style,ve){return style(ve.style());}export const box=css({padding:8});",
    true
)]
#[case(
    concat!("import type {style} from '@vanilla-extract/css';import {css} from '@devup-ui/react';export const box=css({", "padding:8});"),
    true
)]
#[case(
    concat!("import {style} from '@vanilla-extract/css';export const box=style({", "padding:8});const browser=window.document;"),
    false
)]
#[serial]
fn native_origin_controls_keep_devup_scale_or_opt_out_when_identity_is_not_enabled(
    #[case] source: &str,
    #[case] enabled: bool,
) -> TestResult {
    // Given
    reset();
    let option = if enabled {
        super::option()
    } else {
        crate::ExtractOption::default()
    };
    // When
    let output = crate::extract_with_modules("/control.ts", source, option, false, &|_, _| None)?;
    // Then
    if enabled {
        assert!(has_static(&output, "padding", "32px"));
        assert!(!has_static(&output, "padding", "8px"));
    } else {
        assert_eq!(output.styles.len(), 0);
        assert!(output.code.contains("@vanilla-extract/css"));
        assert!(output.code.contains(concat!("style({", "padding:8})")));
    }
    Ok(())
}

#[test]
#[serial]
fn upstream_default_is_not_invented_when_no_real_default_terminal_exists() -> TestResult {
    // Given
    reset();
    let source = "import make from '@vanilla-extract/css';export const box=make({color:'blue'});const browser=window.document;";
    // When
    let output = run("/upstream-default.ts", source, &[])?;
    // Then
    assert_eq!(output.styles.len(), 0);
    let allocator = oxc_allocator::Allocator::default();
    let parsed =
        oxc_parser::Parser::new(&allocator, &output.code, oxc_span::SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    assert!(parsed.program.body.iter().any(|statement| matches!(statement,
        Statement::ImportDeclaration(import) if import.source.value == "@vanilla-extract/css"
            && import.specifiers.iter().flatten().any(|specifier| matches!(specifier,
                ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) if specifier.local.name == "make")))));
    assert!(parsed.program.body.iter().any(|statement| {
        let Statement::ExportDeclaration(export) = statement else { return false };
        let Declaration::VariableDeclaration(declaration) = &export.declaration else { return false };
        declaration.declarations.iter().any(|declarator| {
            let Some(Expression::CallExpression(call)) = &declarator.init else { return false };
            let Expression::Identifier(identifier) = &call.callee else { return false };
            let [Argument::ObjectExpression(object)] = call.arguments.as_slice() else { return false };
            identifier.name == "make" && object.properties.iter().any(|property| matches!(property,
                ObjectPropertyKind::ObjectProperty(property) if property.key.static_name().as_deref() == Some("color")
                    && matches!(&property.value, Expression::StringLiteral(value) if value.value == "blue")))
        })
    }), "{}", output.code);
    Ok(())
}

#[test]
#[serial]
fn actual_barrel_baseline_emits_native_css_when_original_api_identity_is_required() -> TestResult {
    // Given
    reset();
    let source = "import { make } from './W35d-provenance-barrel'\nexport const box = make({ color: 'blue' })\nconst browser = window.document\n";
    let barrel = "export { style as make, createTheme } from '@vanilla-extract/css'\nthrow new Error('unrelated API barrel sibling')\n";
    // When
    let output = run(
        "/W35d-provenance-consumer.ts",
        source,
        &[(
            "./W35d-provenance-barrel",
            "/W35d-provenance-barrel.ts",
            barrel,
        )],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    assert!(!output.code.contains("make("), "{}", output.code);
    assert_import(&output, "./W35d-provenance-barrel");
    assert!(
        output
            .dependencies
            .iter()
            .any(|path| path == "/W35d-provenance-barrel.ts")
    );
    assert!(output.code.contains("window.document"), "{}", output.code);
    Ok(())
}
