use super::*;

const LOCAL: &str = "export function makeCard(){const __devupForwardRef=()=> 'authored';const authored=__devupForwardRef();return styled('section',{p:2});}";
const ROOT: &str = "const __devupForwardRef=()=> 'authored';export const authored=__devupForwardRef();export function makeCard(){return styled('section',{p:2});}";
const PARAMETER: &str = "export function makeCard(__devupForwardRef=()=> 'authored'){const authored=__devupForwardRef();return styled('section',{p:2});}";
const SUFFIXES: &str = "const __devupForwardRef_=1,__devupForwardRef__=2;export function makeCard(){const __devupForwardRef=()=> 'authored';const authored=__devupForwardRef();return styled('section',{p:2});}";
const UNRELATED: &str = "export function authored(){const __devupForwardRef=()=> 'authored';return __devupForwardRef();}export function makeCard(){return styled('section',{p:2});}";

#[rstest]
#[case::local(LOCAL, "__devupForwardRef_")]
#[case::root(ROOT, "__devupForwardRef_")]
#[case::parameter(PARAMETER, "__devupForwardRef_")]
#[case::occupied_suffixes(SUFFIXES, "__devupForwardRef___")]
#[case::unrelated_scope(UNRELATED, "__devupForwardRef")]
#[serial]
fn stylesheet_generated_helper_when_factory_is_uninvoked_preserves_authored_bindings(
    #[case] declaration: &str,
    #[case] helper: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{styled,style}} from '@devup-ui/react';const loop={{}};loop.self=loop;export {{loop}};{declaration}export const box=style({{color:'sienna'}});"
    );
    let resolver = |specifier: &str, _: &str| {
        assert_ne!(specifier, "react", "generated React must not execute");
        None
    };
    // When
    let output = crate::extract_with_modules(
        "/helper-collision.css.ts",
        &source,
        ExtractOption::default(),
        false,
        &resolver,
    )?;
    // Then
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let imported = parsed
        .program
        .body
        .iter()
        .find_map(|statement| {
            let Statement::ImportDeclaration(import) = statement else {
                return None;
            };
            import.specifiers.iter().flatten().find_map(|specifier| {
                let oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named) = specifier
                else {
                    return None;
                };
                (import.source.value == "react" && named.imported.name() == "forwardRef")
                    .then_some(&named.local)
            })
        })
        .ok_or("real forwardRef import missing")?;
    assert_eq!(imported.name, helper);
    let symbol = imported
        .symbol_id
        .get()
        .ok_or("helper import symbol missing")?;
    assert!(
        semantic.nodes().iter().any(|node| {
            let oxc_ast::AstKind::CallExpression(call) = node.kind() else {
                return false;
            };
            let oxc_ast::ast::Expression::Identifier(callee) = &call.callee else {
                return false;
            };
            callee.reference_id.get().is_some_and(|reference| {
                semantic.scoping().get_reference(reference).symbol_id() == Some(symbol)
            })
        }),
        "{}",
        output.code
    );
    if declaration == ROOT {
        assert!(parsed.program.body.iter().any(|statement| {
            let Statement::ExportDeclaration(export) = statement else { return false; };
            let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration else { return false; };
            declaration.declarations.iter().any(|declarator| {
                declarator.id.get_binding_identifier().is_some_and(|id| id.name == "authored")
                    && matches!(&declarator.init, Some(oxc_ast::ast::Expression::StringLiteral(value)) if value.value == "authored")
            })
        }), "{}", output.code);
    } else {
        assert!(
            output.code.contains("__devupForwardRef()"),
            "{}",
            output.code
        );
    }
    assert!(output.styles.iter().any(|value| matches!(value, ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px")));
    Ok(())
}

#[rstest]
#[case::direct("typeof makeCard()")]
#[case::caught("(()=>{try{return typeof makeCard()}catch{return 'red'}})()")]
#[serial]
fn stylesheet_captured_helper_when_factory_is_executed_keeps_rendering_read_error(
    #[case] read: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{styled,style}} from '@devup-ui/react';\nconst loop={{}};loop.self=loop;export {{loop}};\nfunction makeCard(){{const __devupForwardRef=()=> 'authored';return styled('section',{{p:2}});}}\nexport const box=style({{color:{read}}});"
    );
    // When
    let error =
        extract_without_source_map("/captured-read.css.ts", &source, ExtractOption::default())
            .err()
            .ok_or("rendering value dependence was accepted")?
            .to_string();
    // Then
    assert!(error.starts_with("/captured-read.css.ts:3:"), "{error}");
    assert!(
        error.contains("rendering scaffold") && error.contains("Fix:"),
        "{error}"
    );
    Ok(())
}
