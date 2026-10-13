use std::fmt::Write;

use super::*;

const ALIAS_EXPORT: &str =
    "const Card=styled('section',{p:2});export {Card as Named};export default Card;";
const DEFAULT_EXPORT: &str = "export default styled('section',{p:2});";
const MULTIPLE_DECLARATIONS: &str = "export const Card=styled('section',{p:2}), count=2;";
const DESTRUCTURE_WRITE: &str = "({Card}={Card:1});";
const DESTRUCTURE_DEFAULT: &str = "({Card=2}={});";
const WRAPPED_SCAFFOLD: &str = "const value=typeof styled('section',{p:2});";
const LOCAL_SCAFFOLD: &str =
    "function render(){const Card=styled('section',{p:2});return Card;}render();";
const NESTED_SCAFFOLD: &str = "function render(){return styled('section',{p:2});}render();";

#[test]
#[serial]
fn styled_scaffolding_is_retained_when_authored_fallback_exports_component() -> TestResult {
    // Given
    reset();
    let source = "import {styled,style} from '@devup-ui/react';const loop={};loop.self=loop;export {loop};export const Card=styled('section',{p:2});export const box=style({color:'sienna'});";
    let resolver = |specifier: &str, _: &str| {
        assert_ne!(
            specifier,
            "react",
            "generated React must not be resolved for execution: {}",
            std::backtrace::Backtrace::force_capture()
        );
        None
    };
    // When
    let output = crate::extract_with_modules(
        "/own-styled.css.ts",
        source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    // Then
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut setup = String::new();
    let mut has_forward_ref = false;
    for statement in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value == "react"
        {
            for specifier in import.specifiers.iter().flatten() {
                if let oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named) = specifier
                {
                    match named.imported.name().as_str() {
                        "forwardRef" => {
                            write!(setup, "const {}=render=>render;", named.local.name)?;
                            has_forward_ref = true;
                        }
                        "createElement" => {
                            write!(setup, "const {}=h;", named.local.name)?;
                        }
                        _ => panic!("unexpected React helper: {}", named.imported.name()),
                    }
                }
            }
        }
    }
    assert!(has_forward_ref, "{}", output.code);
    let rendered = crate::jsx_semantics_tests::whole::evaluate_compiled(
        &output.code,
        &setup,
        "Card({id:'card'},null)",
    )
    .element;
    assert_eq!(rendered["type"], "section");
    assert_eq!(rendered["props"]["id"], "card");
    let class = rendered["props"]["className"]
        .as_str()
        .ok_or("component class was not a string")?;
    assert_eq!(
        declarations(&output, class),
        vec![("padding".into(), "8px".into())]
    );
    Ok(())
}

#[rstest]
#[case::aliases(ALIAS_EXPORT)]
#[case::default_expression(DEFAULT_EXPORT)]
#[case::multiple_declarations(MULTIPLE_DECLARATIONS)]
#[serial]
fn deferred_export_keeps_original_rendering_when_export_form_changes(
    #[case] declaration: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{styled,style}} from '@devup-ui/react';const loop={{}};loop.self=loop;export {{loop}};{declaration}export const box=style({{color:'sienna'}});"
    );
    // When
    let output =
        extract_without_source_map("/deferred-export.css.ts", &source, ExtractOption::default())?;
    // Then
    assert!(output.code.contains("from \"react\""), "{}", output.code);
    assert!(
        output.code.contains("__devupForwardRef("),
        "{}",
        output.code
    );
    assert!(
        !output.code.contains("__devupDeferredRendering"),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case::shorthand("const values={Card};")]
#[case::write("Card=1;")]
#[case::destructure_write(DESTRUCTURE_WRITE)]
#[case::destructure_default(DESTRUCTURE_DEFAULT)]
#[case::wrapped_scaffold(WRAPPED_SCAFFOLD)]
#[case::local_scaffold(LOCAL_SCAFFOLD)]
#[case::nested_scaffold(NESTED_SCAFFOLD)]
#[serial]
fn deferred_observation_fails_when_authored_code_exercises_other_read_paths(
    #[case] expression: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{styled,style}} from '@devup-ui/react';\nconst loop={{}};loop.self=loop;export {{loop}};\nconst Card=styled('section',{{p:2}});export {{Card}};\n{expression}\nexport const box=style({{color:'sienna'}});"
    );
    // When
    let error = extract_without_source_map(
        "/deferred-observation.css.ts",
        &source,
        ExtractOption::default(),
    )
    .err()
    .ok_or("deferred observation was accepted")?
    .to_string();
    // Then
    assert!(
        error.starts_with("/deferred-observation.css.ts:4:"),
        "{error}"
    );
    assert!(error.contains("rendering scaffold"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn nested_async_base_keeps_authored_flags_when_styled_fallback_compiles() -> TestResult {
    // Given
    reset();
    let source = "import {styled,style} from '@devup-ui/react';const loop={};loop.self=loop;export {loop};export const Card=styled(async ({id})=>await Promise.resolve(id),{p:2});export const box=style({color:'sienna'});";
    // When
    let output =
        extract_without_source_map("/nested-async.css.ts", source, ExtractOption::default())?;
    // Then
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    assert!(semantic.nodes().iter().any(|node| matches!(node.kind(), oxc_ast::AstKind::ArrowFunctionExpression(arrow) if arrow.r#async && matches!(arrow.params.items.first().map(|parameter| &parameter.pattern), Some(oxc_ast::ast::BindingPattern::ObjectPattern(_))))), "{}", output.code);
    Ok(())
}

#[rstest]
#[case::type_read("typeof Card")]
#[case::property_read("Card.displayName")]
#[case::identity_read("Card === Card")]
#[case::caught_read("(()=>{try{return typeof Card}catch{return 'red'}})()")]
#[serial]
fn deferred_rendering_read_fails_when_authored_styles_need_it(#[case] read: &str) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{styled,style}} from '@devup-ui/react';\nconst loop={{}};loop.self=loop;export {{loop}};\nexport const Card=styled('section',{{p:2}});\nexport const box=style({{color:{read}}});"
    );
    // When
    let error =
        extract_without_source_map("/deferred-read.css.ts", &source, ExtractOption::default())
            .err()
            .ok_or("deferred style value was accepted")?
            .to_string();
    // Then
    assert!(error.starts_with("/deferred-read.css.ts:4:"), "{error}");
    assert!(error.contains("rendering scaffold"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn authored_react_import_effect_fails_when_generated_scaffold_is_deferred() -> TestResult {
    // Given
    reset();
    let source = "import {styled,style} from '@devup-ui/react';import {forwardRef} from 'react';const loop={};loop.self=loop;export {loop};export const Card=styled('section',{p:2});export const box=style({color:'sienna'});export const effect=forwardRef;";
    let resolver = |specifier: &str, _: &str| {
        (specifier == "react").then(|| crate::ResolvedModule {
            path: "/authored-react.js".into(),
            code: "const mode=process.env.NODE_ENV;export const forwardRef=mode;".into(),
        })
    };
    // When
    let error = crate::extract_with_modules(
        "/authored-react.css.ts",
        source,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .err()
    .ok_or("authored React effect was ignored")?
    .to_string();
    // Then
    assert!(error.starts_with("/authored-react.js:1:12:"), "{error}");
    assert!(error.contains("process"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}
