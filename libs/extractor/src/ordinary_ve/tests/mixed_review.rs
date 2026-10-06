use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Statement};
use oxc_codegen::{Codegen, Context as CodegenContext, Gen, GenExpr};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::precedence::Precedence;
use rstest::rstest;
use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, extract, has_static};
use crate::{ResolvedModule, extract_with_modules};

/// Evaluate emitted bindings without imports or the preserved host-only sibling.
pub(super) fn emitted_predicate(code: &str, predicate: &str) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{code}");
    let mut script = String::new();
    for mut statement in parsed.program.body {
        let mut codegen = Codegen::new();
        match &mut statement {
            Statement::ImportDeclaration(_) => continue,
            Statement::VariableDeclaration(declaration) => {
                declaration.declarations.retain(|declarator| {
                    !declarator
                        .id
                        .get_identifier_name()
                        .is_some_and(|name| name == "browser")
                });
                if declaration.declarations.is_empty() {
                    continue;
                }
                declaration.print(&mut codegen, CodegenContext::default());
            }
            Statement::ExportDeclaration(export) => {
                let Declaration::VariableDeclaration(declaration) = &export.declaration else {
                    panic!("fixture exports a variable declaration");
                };
                declaration.print(&mut codegen, CodegenContext::default());
            }
            Statement::ExportDefaultDeclaration(export) => {
                script.push_str("const __review_default__ = ");
                export.declaration.to_expression().print_expr(
                    &mut codegen,
                    Precedence::Assign,
                    CodegenContext::default(),
                );
            }
            statement => statement.print(&mut codegen, CodegenContext::default()),
        }
        script.push_str(&codegen.into_source_text());
        script.push_str(";\n");
    }
    let script = format!("(()=>{{{script}return ({predicate});}})()");
    let value = Context::default()
        .eval(Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("emitted binding evaluation failed: {error}\n{script}"));
    value
        .as_boolean()
        .unwrap_or_else(|| panic!("emitted predicate did not return a boolean"))
}

#[rstest]
#[case(
    "if(true){var box=style({color:'red'});}",
    "export const read=()=>box;",
    "read().split(' ').includes('f0_0')"
)]
#[case(
    "{var box=style({color:'red'});}",
    "export const read=()=>box;",
    "read().split(' ').includes('f0_0')"
)]
#[case(
    "if(true){var box=style({color:'red'}), token=createVar();}",
    "export const read=()=>[box,token];",
    "read()[0].split(' ').includes('f0_0') && read()[1]==='var(--var-0-0)'"
)]
#[case(
    "{var box=style({color:'red'}), token=createVar();}",
    "export const read=()=>[box,token];",
    "read()[0].split(' ').includes('f0_0') && read()[1]==='var(--var-0-0)'"
)]
#[serial]
fn module_vars_survive_when_a_known_statement_owns_native_initialization(
    #[case] owner: &str,
    #[case] consumer: &str,
    #[case] predicate: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{style,createVar}} from '@vanilla-extract/css';{owner}{consumer}const browser=window.document;"
    );
    // When
    let output = extract("ts", &source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "red"));
    assert_preserved(&output.code, consumer);
    assert_preserved(&output.code, "const browser=window.document;");
    assert!(
        emitted_predicate(&output.code, predicate),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn mixed_stylesheet_import_succeeds_without_executing_preserved_runtime(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    let path = format!("/review-producer.{extension}");
    let producer = "import {style} from '@vanilla-extract/css';\nexport const base=style({color:'red'});\nthrow new Error('preserved producer executed');";
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./producer").then(|| ResolvedModule {
            path: path.clone(),
            code: producer.to_string(),
        })
    };
    let source = "import {style} from '@vanilla-extract/css';import {base} from './producer';export const box=style([base,{color:'blue'}]);const browser=window.document;";
    // When
    let output = extract_with_modules("/mixed.ts", source, super::option(), false, &resolver)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(!has_static(&output, "color", "red"));
    super::demand_support::assert_import(&output, "./producer");
    Ok(())
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn unrelated_throwing_initializer_is_preserved_when_only_native_roots_are_required(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';const unrelated=JSON.parse('not JSON');export const box=style({color:'red'});";
    // When
    let output = extract(extension, source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "red"));
    assert_preserved(&output.code, "const unrelated=JSON.parse('not JSON');");
    Ok(())
}

#[test]
#[serial]
fn original_duplicate_let_is_located_when_a_private_native_helper_is_removed() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';\nfunction unused(){\nlet repeated;\nlet repeated;\nreturn style({});\n}\nexport const box=style({color:'red'});\nconst browser=window.document;";
    let locations = [
        source
            .find("repeated;")
            .unwrap_or_else(|| panic!("fixture first declaration missing")),
        source
            .rfind("repeated;")
            .unwrap_or_else(|| panic!("fixture duplicate declaration missing")),
    ]
    .map(|offset| crate::locate("/mixed.ts", source, offset));
    // When
    let result = extract("ts", source);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("duplicate let unexpectedly succeeded"))
        .to_string();
    assert!(
        locations.iter().any(|place| error.contains(place)),
        "{error}"
    );
    assert!(error.contains("repeated"), "{error}");
    assert!(
        error.contains("SyntaxError") || error.contains("Cannot parse source"),
        "{error}"
    );
    assert!(error.contains("Fix:"), "{error}");
}

#[rstest]
#[case(
    concat!("const shared=", "{value:7};return {left:shared,right:shared};"),
    "result.left===result.right && result.left.value===7"
)]
#[case("return [,];", "result.length===1 && !(0 in result)")]
#[case(
    "return {['__proto__']:'own'};",
    "Object.prototype.hasOwnProperty.call(result,'__proto__') && result['__proto__']==='own'"
)]
#[serial]
fn emitted_capture_keeps_observable_shape_when_native_roots_return_plain_data(
    #[case] body: &str,
    #[case] predicate: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';export const result=(()=>{{style({{color:'red'}});{body}}})();const browser=window.document;"
    );
    // When
    let output = extract("ts", &source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "red"));
    assert_preserved(&output.code, "const browser=window.document;");
    assert!(
        emitted_predicate(&output.code, predicate),
        "{}",
        output.code
    );
    Ok(())
}
