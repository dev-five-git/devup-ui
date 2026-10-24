use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, ObjectExpression},
    builder::AstBuilder,
};
use oxc_ast_visit::{VisitMut, walk_mut};
use rustc_hash::FxHashMap;

use super::{PropsRulesError, branches::Captures, capture};

pub(super) struct Direct<'b, 'p, 'a> {
    pub builder: &'b AstBuilder<'a>,
    pub name: &'p str,
    pub scope: Expression<'a>,
    pub next: usize,
}

impl<'a> Captures<'a> for Direct<'_, '_, 'a> {
    fn object(
        &mut self,
        source: &ObjectExpression<'a>,
        local: &str,
    ) -> Result<(Expression<'a>, Vec<Expression<'a>>), PropsRulesError> {
        let builder = self.builder;
        let source = Expression::ObjectExpression(oxc_allocator::Box::new_in(
            source.clone_in_with_semantic_ids(builder.allocator()),
            builder,
        ));
        let Some(mut prepared) =
            crate::extractor::styled_record::prepare(builder, &source, &self.scope)
        else {
            let evaluated = source.clone_in_with_semantic_ids(builder.allocator());
            return Ok((source, vec![evaluated]));
        };
        super::super::records::reserve(builder, &mut self.scope, &prepared.bindings);
        let start = self.next;
        self.next += prepared.bindings.len();
        let names = prepared
            .bindings
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (name.clone(), start + index))
            .collect();
        let mut slots = Slots {
            builder,
            vector: self.name,
            names,
        };
        slots.visit_expression(&mut prepared.rules);
        let values = prepared
            .bindings
            .into_iter()
            .enumerate()
            .map(|(index, (_, mut value))| {
                slots.visit_expression(&mut value);
                capture::assign(builder, capture::slot(builder, local, start + index), value)
            })
            .collect();
        Ok((prepared.rules, values))
    }

    fn theme(&self, source: &Expression<'a>) -> Result<(), PropsRulesError> {
        if crate::css_utils::reads_unmapped_theme(source) {
            return Err(PropsRulesError::Theme(
                crate::css_utils::unmapped_theme_error("styled", source),
            ));
        }
        Ok(())
    }

    fn discarded_object(&self, source: &Expression<'a>) -> Result<Expression<'a>, PropsRulesError> {
        Ok(source.clone_in_with_semantic_ids(self.builder.allocator()))
    }
}

struct Slots<'b, 'p, 'a> {
    builder: &'b AstBuilder<'a>,
    vector: &'p str,
    names: FxHashMap<String, usize>,
}

impl<'a> VisitMut<'a> for Slots<'_, '_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if let Expression::Identifier(identifier) = expression
            && let Some(index) = self.names.get(identifier.name.as_str())
        {
            *expression = Expression::ComputedMemberExpression(capture::slot(
                self.builder,
                self.vector,
                *index,
            ));
            return;
        }
        walk_mut::walk_expression(self, expression);
    }
}
