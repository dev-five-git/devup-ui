use crate::visit::DevupVisitor;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, Statement};
use oxc_ast_visit::VisitMut;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use std::rc::Rc;

pub(super) fn produced<'a>(
    allocator: &'a Allocator,
    source: &'a str,
) -> (DevupVisitor<'a>, Expression<'a>) {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let mut parsed = Parser::new(allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = SemanticBuilder::new().build(&parsed.program);
    assert_eq!(semantic.diagnostics.len(), 0);
    let mut visitor = DevupVisitor::new(allocator, "a.tsx", "@devup-ui/react", vec![], None);
    visitor.source = Some(source);
    visitor.reuse_scoping(Some(Rc::new(semantic.semantic.into_scoping())));
    visitor.visit_program(&mut parsed.program);
    assert_eq!(visitor.errors, vec![]);
    let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.pop() else {
        panic!("source must end with the retained unknown-tag template")
    };
    let expression = statement.expression.clone_in_with_semantic_ids(allocator);
    (visitor, expression)
}
