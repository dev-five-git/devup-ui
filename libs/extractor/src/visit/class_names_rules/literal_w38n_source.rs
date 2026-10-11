use super::literal_w38n_inventory::{Inventory, inventory};
use crate::gen_class_name::roots::ClassPayload;
use crate::visit::{DevupVisitor, class_names_parts::UncapturedSource};
use crate::{ErrorDisposition, ExtractStyleProp};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::AstKind;
use oxc_ast_visit::VisitMut;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use std::{error::Error, rc::Rc};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn Error>>;

pub(super) struct Observation {
    pub errors: Vec<(u32, String)>,
    pub disposition: ErrorDisposition,
    pub inventory: Vec<Inventory>,
    pub classes: String,
}

pub(super) fn observe(source: &str) -> TestResult<Observation> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    assert_eq!(parsed.program.body.len(), 1);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(semantic.diagnostics.len(), 0);
    let statement = semantic
        .semantic
        .nodes()
        .kind(parsed.program.body[0].node_id())
        .as_expression_statement()
        .ok_or("template statement")?
        .clone_in_with_semantic_ids(&allocator);
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    visitor.source = Some(source);
    visitor.reuse_scoping(Some(Rc::new(semantic.semantic.into_scoping())));
    visitor.visit_program(&mut parsed.program);
    assert_eq!(visitor.errors, vec![]);
    let expression = statement.expression.clone_in_with_semantic_ids(&allocator);
    let tag = AstKind::from_expression(&expression)
        .as_tagged_template_expression()
        .ok_or("original template")?;
    let text =
        crate::css_utils::literal::CssText::from_template(&visitor.ast, &tag.quasi, Some(source));
    let object = text.scoped_object(&visitor.ast, 0..text.text.len(), false);
    let props = visitor
        .literal_scope_local(&object, &UncapturedSource)
        .ok_or("source-fed mixin envelope")?;
    let mut props: Vec<ExtractStyleProp<'_>> = props
        .iter()
        .map(|prop| prop.clone_payload_in(&allocator, ClassPayload::clone_expression))
        .collect();
    let inventory = inventory(&props);
    css::debug::set_debug(true);
    let classes = crate::gen_class_name::gen_class_names(&visitor.ast, &mut props, None, None);
    css::debug::set_debug(false);
    let classes = crate::utils::readable_code(&classes.ok_or("surviving class selection")?);
    Ok(Observation {
        errors: visitor.errors,
        disposition: visitor.error_disposition,
        inventory,
        classes,
    })
}

pub(super) fn offset(source: &str, token: &str) -> TestResult<u32> {
    Ok(u32::try_from(
        source.find(token).ok_or("authored diagnostic token")?,
    )?)
}

pub(super) fn invalid(code: &str) -> String {
    format!(
        "`styleOrder()` cannot use `{code}` at build time: an explicit styleOrder must be an ECMAScript Number integer from 1 to 254 or canonical decimal string without signs, spaces or leading zeros"
    )
}
