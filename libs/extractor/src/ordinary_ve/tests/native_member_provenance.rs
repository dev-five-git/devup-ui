use super::{
    consumer_support::located_failure,
    demand_support::{TestResult, reset, run},
};
use oxc_ast::ast::{Argument, Declaration, Expression, ImportDeclarationSpecifier, Statement};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::unreadable(
    "import '@vanilla-extract/css';export {style as make} from './missing';",
    "import * as ns from './bad';\nns.make({});",
    ("/entry.ts:2:1", "native re-export `./missing` cannot be read")
)]
#[case::changed(
    "import {style} from '@vanilla-extract/css';\nconst make=style;\nmake.extra=1;\nexport {make};",
    "import * as ns from './bad';\nns['make']({});",
    ("/bad.ts:3:1", "native API alias may be changed outside its exact initialization slice")
)]
#[serial]
fn public_failure_keeps_the_correct_origin_when_an_invalid_namespace_member_is_called(
    #[case] bad: &str,
    #[case] source: &str,
    #[case] expected: (&str, &str),
) {
    // Given
    reset();
    let modules = [("./bad", "/bad.ts", bad)];
    // When
    let result = run("/entry.ts", source, &modules);
    // Then
    let Err(error) = &result else {
        panic!("required failure published output")
    };
    let error = error.to_string();
    assert!(error.starts_with(&format!("{}:", expected.0)), "{error}");
    assert!(!error.contains("/entry.ts:1:13"), "{error}");
    assert!(!error.contains("/bad.ts:2:1"), "{error}");
    located_failure(result, expected.0, expected.1);
}

#[test]
#[serial]
fn native_type_error_is_left_to_typescript_when_the_value_read_has_no_runtime_binding() -> TestResult
{
    // Given
    reset();
    let source = "import {type style} from '@vanilla-extract/css';\nexport const box=style({});const browser=window.document;";
    // When
    let output = run("/type-input.ts", source, &[])?;
    // Then
    assert_eq!(output.styles.len(), 0);
    assert_eq!(output.dependencies.len(), 0);
    let allocator = oxc_allocator::Allocator::default();
    let parsed =
        oxc_parser::Parser::new(&allocator, &output.code, oxc_span::SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ImportDeclaration(import) = &parsed.program.body[0] else {
        panic!("retained type import missing")
    };
    assert_eq!(import.source.value, "@vanilla-extract/css");
    let [ImportDeclarationSpecifier::ImportSpecifier(specifier)] = import
        .specifiers
        .as_deref()
        .ok_or("retained type specifier missing")?
        .as_slice()
    else {
        panic!("retained type import shape changed")
    };
    assert!(specifier.import_kind.is_type());
    assert_eq!(specifier.imported.name(), "style");
    assert_eq!(specifier.local.name, "style");
    let Statement::ExportDeclaration(export) = &parsed.program.body[1] else {
        panic!("application value declaration missing")
    };
    let Declaration::VariableDeclaration(declaration) = &export.declaration else {
        panic!("application value declaration changed")
    };
    let Some(Expression::CallExpression(call)) = &declaration.declarations[0].init else {
        panic!("application style call was not retained")
    };
    assert!(
        matches!(&call.callee, Expression::Identifier(identifier) if identifier.name == "style")
    );
    let [Argument::ObjectExpression(object)] = call.arguments.as_slice() else {
        panic!("application style call argument changed")
    };
    assert_eq!(object.properties.len(), 0);
    Ok(())
}
