use super::{ModuleScope, scalar_reads};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;
use std::rc::Rc;

#[rstest]
#[case(
    "const made={ width:'13px',child:{ n:1 }};watch(made.child);made.width;",
    Some("\"13px\"")
)]
#[case(
    "const made={ width:'13px',child:{ n:1 }};watch(made.child);made['width'];",
    Some("\"13px\"")
)]
#[case(
    "const made={ width:13,child:{ n:1 }};watch(made.child);made.width;",
    Some("13")
)]
#[case(
    "const made={ width:true,child:{ n:1 }};watch(made.child);made.width;",
    Some("true")
)]
#[case(
    "const made={ width:null,child:{ n:1 }};watch(made.child);made.width;",
    Some("null")
)]
#[case("const made={ width:1e999 };made.width;", None)]
#[case(
    "const made={ width:'13px',child:{ n:1 }};watch(made.child);(made['width' as const] as string);",
    Some("\"13px\"")
)]
#[case("declare const made:{ width:string };made.width;", None)]
#[case(
    "declare const original:{ width:string };const made={ ...original };made.width;",
    None
)]
#[case("const made={ width:{ n:1 }};made.width;", None)]
#[case("const made={ child:{ n:1 }};made.width;", None)]
#[case("const made={width:'13px',child:[1]};made.width;", None)]
#[case("const made={width:'13px',get child(){return 1}};made.width;", None)]
#[case("const made={width:'13px',child(){}};made.width;", None)]
#[case("const made={width:'13px',['child']:1};made.width;", None)]
#[case("const made={width:'13px',toJSON:1};made.width;", None)]
#[case("const made={ width:'13px',child:{ toString:1 }};made.width;", None)]
#[case("const made={width:'13px',child:unknown};made.width;", None)]
#[case("const made={...{width:'13px'}};made.width;", None)]
#[case(
    "const original=Object.freeze({width:'13px'});const made={...original};made.width;",
    None
)]
#[case(
    "const original={ width:'13px',child:{ n:1 }};const made={ ...original };made.width;",
    None
)]
#[case(
    "const original={width:'13px'};const alias=original;const made={...original};made.width;",
    None
)]
#[case(
    "const original={width:'13px'};const made={...original};original.width='2px';made.width;",
    Some("\"13px\"")
)]
#[case(
    "const original={width:'13px'};const made={...original};watch(original.width);watch(original);made.width;",
    Some("\"13px\"")
)]
#[case(
    "const original={width:'13px'};const made={...original};original.custom();made.width;",
    Some("\"13px\"")
)]
#[case(
    "const original={width:'13px'};watch(original);const made={...original};made.width;",
    None
)]
#[case(
    "const original={width:'13px'};const made={...original};function later(){watch(original)}made.width;",
    None
)]
#[case("const made=Object.freeze(unknown);made.width;", None)]
#[case("const made=unknown();made.width;", None)]
#[case("let made={width:'13px'};made.width;", None)]
#[case("const made={width:'13px'};made={width:'2px'};made.width;", None)]
#[case("export const made={width:'13px'};made.width;", None)]
#[case("const made={width:'13px'};export {made};made.width;", None)]
#[case(
    "const made={width:'13px'};const f=()=>made.width;f();",
    Some("\"13px\"")
)]
#[case("const made={width:'13px'};const f=()=>made.width;watch(f);f();", None)]
#[case(
    "const made={width:'13px'};const f=()=>made.width;export {f};f();",
    None
)]
#[case("const made={width:'13px'};const f=()=>made.width;f=()=>2;f();", None)]
#[case("const made={width:'13px'};const f=()=>unknown.width;f();", None)]
#[case("const made={width:'13px'};const f=()=>made.child.width;f();", None)]
#[case("const made={width:'13px'};const f=()=>made[key];f();", None)]
#[case("const made={width:'13px'};const f=()=>made?.width;f();", None)]
#[case("const made={width:'13px'};const f=()=>13;f();", None)]
#[case("const made={width:'13px'};const f=(x)=>made.width;f();", None)]
#[case(
    "const made={ width:'13px',child:{ n:1 }};watch(made);made.width;",
    None
)]
#[case(
    "const made=Object.freeze({ width:'13px',child:{ n:1 }});watch(made);made.width;",
    Some("\"13px\"")
)]
#[case(
    "const made={ width:'13px',child:{ n:1 }};const alias=made;made.width;",
    None
)]
#[case(
    "const made={ width:'13px',child:{ n:1 }};made.child.n=2;made.width;",
    None
)]
#[case(
    "const made={ width:'13px',child:{ n:1 }};watch(made[key]);made.width;",
    None
)]
#[case(
    "const made={ width:'13px',child:{ n:1 }};function later(){ watch(made.child) }made.width;",
    None
)]
fn scalar_projection_when_real_source_proofs_allow_or_decline_a_slot(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) {
    // Given
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let expression = match parsed
        .program
        .body
        .last()
        .unwrap_or_else(|| panic!("fixture read"))
    {
        Statement::ExpressionStatement(statement) => &statement.expression,
        _ => panic!("fixture ends in a read"),
    };
    let read = scalar_reads::Read::new(expression, semantic.scoping())
        .unwrap_or_else(|| panic!("bounded descriptor"));
    let mut scope = ModuleScope::new("/src/App.tsx", &parsed.program, None);
    scope.uses = Some(Rc::new(crate::mutations::uses(
        &parsed.program,
        &|_| false,
        None,
    )));
    // When
    let values = scalar_reads::resolve(&scope, &[read]);
    // Then
    assert_eq!(
        values
            .values()
            .next()
            .and_then(super::Constant::js_literal)
            .as_deref(),
        expected,
        "{source}"
    );
}

#[rstest]
#[case("const made={};made[key];")]
#[case("const made={};made.child.width;")]
#[case("const made={};made?.width;")]
#[case("const f=()=>1;f(1);")]
#[case("const f=()=>1;f?.();")]
#[case("const made={f:()=>1};made.f();")]
#[case("unknown.width;")]
#[case("unknown();")]
fn scalar_descriptor_when_the_read_is_outside_the_bounded_grammar_is_absent(#[case] source: &str) {
    // Given
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let Statement::ExpressionStatement(statement) = parsed
        .program
        .body
        .last()
        .unwrap_or_else(|| panic!("fixture read"))
    else {
        panic!("fixture read")
    };
    // When
    let read = scalar_reads::Read::new(&statement.expression, semantic.scoping());
    // Then
    assert!(read.is_none(), "{source}");
}

#[rstest]
#[case("const made={width:'13px'};const f=()=>made.width;")]
#[case("const made={width:'13px'};function f(){return made.width}")]
#[case("const made={width:'13px'};class View {width=made.width}")]
#[case("const made={width:'13px'};class View {static width=made.width}")]
#[case("function f(){const made={width:'13px'};return made.width}")]
fn scalar_projection_when_the_read_is_deferred_is_absent(#[case] source: &str) {
    use oxc_ast_visit::{Visit, walk};
    struct Reads<'s> {
        scoping: &'s oxc_semantic::Scoping,
        reads: Vec<scalar_reads::Read>,
    }
    impl<'a> Visit<'a> for Reads<'_> {
        fn visit_expression(&mut self, expression: &oxc_ast::ast::Expression<'a>) {
            self.reads
                .extend(scalar_reads::Read::new(expression, self.scoping));
            walk::walk_expression(self, expression);
        }
    }
    // Given
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let mut reads = Reads {
        scoping: semantic.scoping(),
        reads: Vec::new(),
    };
    reads.visit_program(&parsed.program);
    assert_eq!(reads.reads.len(), 1);
    let scope = ModuleScope::new("/src/App.tsx", &parsed.program, None);
    // When
    let values = scalar_reads::resolve(&scope, &reads.reads);
    // Then
    assert_eq!(values.len(), 0);
}
