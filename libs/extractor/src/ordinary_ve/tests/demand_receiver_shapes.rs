use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPattern, Declaration, Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serial_test::serial;

use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::{assert_consumed, has_static};
use crate::ordinary_ve::selection::demand::receiver_fixtures::{
    BASE, E1_SOURCE, E2_SOURCE, E3_SOURCE, P1, P2, P3,
};
use crate::{ExtractOutput, ExtractStyleValue};

fn assert_static_box(output: &ExtractOutput) {
    assert!(has_static(output, "padding", "8px"));
    assert_consumed(output);
    let padding: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "padding" => {
                Some(Some(style.value.as_str()))
            }
            ExtractStyleValue::Dynamic(style) if style.property() == "padding" => Some(None),
            ExtractStyleValue::Static(_)
            | ExtractStyleValue::Dynamic(_)
            | ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::Keyframes(_) => None,
        })
        .collect();
    assert_eq!(padding, vec![Some("8px")]);
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let boxes: Vec<_> = parsed
        .program
        .body
        .iter()
        .filter_map(|statement| {
            let Statement::ExportDeclaration(export) = statement else {
                return None;
            };
            let Declaration::VariableDeclaration(declaration) = &export.declaration else {
                return None;
            };
            Some(declaration.declarations.iter().filter(|declarator| {
                matches!(&declarator.id,
            BindingPattern::BindingIdentifier(binding) if binding.name == "box")
            }))
        })
        .flatten()
        .collect();
    let [declarator] = boxes.as_slice() else {
        panic!("expected one exported box")
    };
    assert!(matches!(
        &declarator.init,
        Some(Expression::StringLiteral(_))
    ));
}

#[test]
#[serial]
fn padding_is_static_when_an_imported_ordinary_method_reads_receiver_space() -> TestResult {
    // Given
    reset();
    // When
    let output = run(
        "/receiver-consumer.ts",
        E1_SOURCE,
        &[("./receiver", "/receiver.ts", P1)],
    )?;
    // Then
    assert_static_box(&output);
    assert_import(&output, "./receiver");
    assert!(
        output
            .dependencies
            .iter()
            .any(|path| path == "/receiver.ts")
    );
    Ok(())
}

#[test]
#[serial]
fn unknown_receiver_input_errors_at_the_original_producer_when_read_returns_this() -> TestResult {
    // Given
    reset();
    let place = crate::locate(
        "/receiver.ts",
        P2,
        P2.find("window.document").ok_or("missing browser")?,
    );
    let consumer = crate::locate(
        "/whole-receiver-consumer.ts",
        E2_SOURCE,
        E2_SOURCE
            .find("tokens.read().space")
            .ok_or("missing consumer read")?,
    );
    // When
    let result = run(
        "/whole-receiver-consumer.ts",
        E2_SOURCE,
        &[("./receiver", "/receiver.ts", P2)],
    );
    // Then
    let error = match result {
        Ok(output) => panic!(
            "required receiver/browser input silently dropped: {}",
            output.code
        ),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&format!("{place}:")), "{error}");
    assert!(
        error.contains("cannot use `window.document` at build time"),
        "{error}"
    );
    assert!(
        error.contains("this required expression needs an exact, static input"),
        "{error}"
    );
    assert!(
        error.contains("Fix: provide an exact static value or CSS variable for this expression"),
        "{error}"
    );
    assert!(!error.contains(&format!("{consumer}:")), "{error}");
    Ok(())
}

#[test]
#[serial]
fn padding_is_static_when_wrapped_forwarding_selects_only_the_imported_base_leaf() -> TestResult {
    // Given
    reset();
    let modules = [
        ("./forward", "/forward.ts", P3),
        ("./base", "/base.ts", BASE),
    ];
    // When
    let output = run("/forward-consumer.ts", E3_SOURCE, &modules)?;
    // Then
    assert_static_box(&output);
    assert_import(&output, "./forward");
    for required in ["/forward.ts", "/base.ts"] {
        assert!(output.dependencies.iter().any(|path| path == required));
    }
    Ok(())
}
