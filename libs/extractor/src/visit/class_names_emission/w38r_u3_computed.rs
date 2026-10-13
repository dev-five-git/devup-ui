use super::*;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast_visit::{Visit, VisitMut};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use rstest::rstest;
use serial_test::serial;
use std::{collections::HashMap, error::Error, rc::Rc};

struct Original<'a> {
    allocator: &'a Allocator,
    span: oxc_span::Span,
    leaf: Option<Expression<'a>>,
}

impl<'a> Visit<'a> for Original<'a> {
    fn visit_expression(&mut self, value: &Expression<'a>) {
        if value.span() == self.span {
            assert_eq!(usize::from(self.leaf.is_some()), 0);
            self.leaf = Some(value.clone_in_with_semantic_ids(self.allocator));
        }
        oxc_ast_visit::walk::walk_expression(self, value);
    }
}

#[rstest]
#[case::empty("")]
#[case::external("ext-a")]
#[serial]
fn w38r_u3_computed_when_actual_capture_is_read_and_replayed_preserves_key_and_getter(
    #[case] external: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: the genuine authored computed leaf, parsed with semantic identities.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let authored = "import {ClassNames} from '@emotion/react';const render=external=>{const mark=(n,v)=>(trace.push(n),v);const object={get value(){trace.push('getter');return external}};return <ClassNames>{({css,cx})=>cx(css({styleOrder:2,color:'red'}),object[mark('key','value')])}</ClassNames>};";
    let aliased = crate::import_alias_visit::transform_import_aliases_with_edits(
        authored,
        "a.tsx",
        "@devup-ui/react",
        &HashMap::from([(
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        )]),
    );
    let allocator = Allocator::default();
    let source = allocator.alloc_str(&aliased.code);
    let text = "object[mark('key','value')]";
    let start = u32::try_from(source.find(text).ok_or("authored computed leaf")?)?;
    let span = oxc_span::Span::new(start, start + u32::try_from(text.len())?);
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(semantic.diagnostics.len(), 0);
    let mut original = Original {
        allocator: &allocator,
        span,
        leaf: None,
    };
    original.visit_program(&parsed.program);
    let mut leaf = original.leaf.ok_or("original parsed computed leaf")?;
    let Expression::ComputedMemberExpression(member) = &leaf else {
        panic!("authored computed member")
    };
    let object_span = member.object.span();
    let key_span = member.expression.span();
    let Expression::Identifier(object) = &member.object else {
        panic!("original object reference")
    };
    let reference_id = object
        .reference_id
        .get()
        .ok_or("object semantic identity")?;
    let scoping = Rc::new(semantic.semantic.into_scoping());
    let symbol_id = scoping
        .get_reference(reference_id)
        .symbol_id()
        .ok_or("object symbol")?;
    assert_eq!(scoping.symbol_name(symbol_id), "object");
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    visitor.source = Some(source);
    visitor.reuse_scoping(Some(Rc::clone(&scoping)));
    visitor.visit_program(&mut parsed.program);
    assert_eq!(visitor.errors, vec![]);
    // When: replay only the retained leaf from unchanged, real capture machinery.
    let mut captures = Vec::new();
    visitor.capture_style_argument(&mut leaf, &mut captures);
    assert_eq!(visitor.errors, vec![]);
    let owner = NonemptyCaptures::from_actual(captures).ok_or("actual capture owner")?;
    let payloads: Vec<_> = std::iter::once(&owner.first)
        .chain(owner.rest.iter())
        .filter(|(_, value)| value.span() == span)
        .collect();
    assert_eq!(payloads.len(), 1);
    let (name, retained) = payloads[0];
    let Expression::Identifier(substitute) = &leaf else {
        panic!("real capture substitution")
    };
    assert_eq!(substitute.name.as_str(), name);
    let source: CapturedSource<'_, '_> = &owner;
    let mut class = source
        .class(&visitor.ast, retained)
        .ok_or("retained computed class")?;
    class.read_in(&visitor.style_values, &visitor.ast);
    let replay = class.clone_source_expression(&allocator);
    let Expression::ComputedMemberExpression(member) = &replay else {
        panic!("replayed computed member")
    };
    assert_eq!(member.span, span);
    assert_eq!(member.object.span(), object_span);
    assert_eq!(member.expression.span(), key_span);
    assert!(!member.optional);
    let Expression::Identifier(object) = &member.object else {
        panic!("replayed original object")
    };
    assert_eq!(object.reference_id.get(), Some(reference_id));
    assert_eq!(
        scoping.get_reference(reference_id).symbol_id(),
        Some(symbol_id)
    );
    // Then: original key and getter each execute once, in authored order.
    let script = format!(
        "const trace=[];const external={};const mark=(n,v)=>(trace.push(n),v);const object={{get value(){{trace.push('getter');return external}}}};JSON.stringify([({}),trace]);",
        serde_json::json!(external),
        readable_code(&replay),
    );
    let mut context = boa_engine::Context::default();
    let result = context.eval(boa_engine::Source::from_bytes(script.as_bytes()))?;
    let json = result.to_string(&mut context)?.to_std_string_escaped();
    let actual: (String, Vec<String>) = serde_json::from_str(&json)?;
    assert_eq!(
        actual,
        (
            external.to_string(),
            vec!["key".to_string(), "getter".to_string()]
        )
    );
    Ok(())
}
