//! Capture-only hygiene for the compiler-owned React import and its call sites.
use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::{
    AstKind,
    ast::{CallExpression, Expression, Program, Str},
};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_semantic::SemanticBuilder;
use oxc_span::SPAN;
use rustc_hash::FxHashSet;

use crate::extractor::extract_style_from_styled::FORWARD_REF;

pub(super) fn forward_ref<'a>(
    ast: &oxc_ast::builder::AstBuilder<'a>,
    program: &mut Program<'a>,
) -> String {
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    let scoping = semantic.scoping();
    let captured = semantic.nodes().iter().any(|node| {
        let AstKind::CallExpression(call) = node.kind() else {
            return false;
        };
        let Some(callee) = generated_callee(call) else {
            return false;
        };
        callee
            .reference_id
            .get()
            .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_some())
    });
    let mut name = FORWARD_REF.to_string();
    if captured {
        let taken: FxHashSet<_> = scoping
            .symbol_names()
            .chain(
                scoping
                    .root_unresolved_references()
                    .keys()
                    .map(oxc_ast::ast::Ident::as_str),
            )
            .collect();
        while taken.contains(name.as_str()) {
            name.push('_');
        }
        Rename { ast, name: &name }.visit_program(program);
    }
    name
}

fn generated_callee<'b, 'a>(
    call: &'b CallExpression<'a>,
) -> Option<&'b oxc_ast::ast::IdentifierReference<'a>> {
    match &call.callee {
        Expression::Identifier(callee) if call.span == SPAN && callee.name == FORWARD_REF => {
            Some(callee)
        }
        _ => None,
    }
}

struct Rename<'a, 's> {
    ast: &'s oxc_ast::builder::AstBuilder<'a>,
    name: &'s str,
}

impl<'a> VisitMut<'a> for Rename<'a, '_> {
    fn visit_call_expression(&mut self, call: &mut CallExpression<'a>) {
        if generated_callee(call).is_some()
            && let Expression::Identifier(callee) = &mut call.callee
        {
            callee.name = Str::from_in(self.name, self.ast.allocator()).into();
        }
        walk_mut::walk_call_expression(self, call);
    }
}
