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
            assert!(self.leaf.is_none());
            self.leaf = Some(value.clone_in_with_semantic_ids(self.allocator));
        }
        oxc_ast_visit::walk::walk_expression(self, value);
    }
}

fn evaluate_once(
    value: &Expression<'_>,
    external: &str,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
    let script = format!(
        "const trace=[];const external={};const object={{get value(){{trace.push('getter');return external}}}};JSON.stringify([({}),trace]);",
        serde_json::json!(external),
        readable_code(value),
    );
    let mut context = boa_engine::Context::default();
    let result = context.eval(boa_engine::Source::from_bytes(script.as_bytes()))?;
    let json = result.to_string(&mut context)?.to_std_string_escaped();
    Ok(serde_json::from_str(&json)?)
}

#[rstest]
#[case::empty("")]
#[case::nonempty("ext-a")]
#[serial]
fn w38q_leaf_when_actual_static_capture_is_replayed_preserves_original_semantics(
    #[case] external: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: alias transformation precedes parsing and building real semantic nodes.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let authored = "import {ClassNames} from '@emotion/react';const render=external=>{const object={get value(){trace.push('getter');return external}};return <ClassNames>{({css,cx})=>cx(css({styleOrder:2,color:'red'}),object.value)}</ClassNames>};";
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
    let start = u32::try_from(source.find("object.value").ok_or("authored member")?)?;
    let span = oxc_span::Span::new(start, start + 12);
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
    let mut leaf = original.leaf.ok_or("pre-visitor parsed member")?;
    let Expression::StaticMemberExpression(member) = &leaf else {
        panic!("original static member")
    };
    assert_eq!(member.span, span);
    assert_eq!(member.property.name, "value");
    assert!(!member.optional);
    let Expression::Identifier(object) = &member.object else {
        panic!("original object reference")
    };
    assert_eq!(object.name, "object");
    let object_span = object.span;
    let property_span = member.property.span;
    assert_eq!(object_span, oxc_span::Span::new(start, start + 6));
    assert_eq!(property_span, oxc_span::Span::new(start + 7, start + 12));
    let reference_id = object.reference_id.get().ok_or("semantic reference ID")?;
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
    assert_ne!(visitor.styles.len(), 0);
    // When: unchanged capture produces the actual owner; replay only its retained payload.
    let mut captures = Vec::new();
    visitor.capture_style_argument(&mut leaf, &mut captures);
    assert_eq!(visitor.errors, vec![]);
    let owner = NonemptyCaptures::from_actual(captures).ok_or("genuine nonempty capture owner")?;
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
    let Expression::StaticMemberExpression(member) = retained else {
        panic!("retained static payload")
    };
    assert_eq!(member.span, span);
    assert_eq!(member.property.span, property_span);
    assert_eq!(member.property.name, "value");
    let Expression::Identifier(object) = &member.object else {
        panic!("retained object reference")
    };
    assert_eq!(object.span, object_span);
    assert_eq!(object.reference_id.get(), Some(reference_id));
    let source: CapturedSource<'_, '_> = &owner;
    let mut class = source
        .class(&visitor.ast, retained)
        .ok_or("captured static class")?;
    class.read_in(&visitor.style_values, &visitor.ast);
    let replay = class.clone_source_expression(&allocator);
    let Expression::StaticMemberExpression(member) = &replay else {
        panic!("replayed static member")
    };
    assert_eq!(member.span, span);
    assert_eq!(member.property.span, property_span);
    assert_eq!(member.property.name, "value");
    assert!(!member.optional);
    let Expression::Identifier(object) = &member.object else {
        panic!("replayed object reference")
    };
    assert_eq!(object.span, object_span);
    assert_eq!(object.reference_id.get(), Some(reference_id));
    assert_eq!(
        scoping.get_reference(reference_id).symbol_id(),
        Some(symbol_id)
    );
    // Then: only the replay leaf runs, once, in a separate original environment.
    let actual = evaluate_once(&replay, external)?;
    assert_eq!(actual, (external.to_string(), vec!["getter".to_string()]));
    Ok(())
}
