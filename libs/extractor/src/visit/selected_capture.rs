//! Native member selection carries a primitive branch tag and evaluated leaves.

use super::DevupVisitor;
use super::branch_capture::BranchReads;
use super::capture::Captured;
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKind, Str};
use oxc_ast_visit::VisitMut;
use oxc_span::SPAN;
use rustc_hash::FxHashMap;

impl<'a> DevupVisitor<'a> {
    fn selected_slot(&self, name: &str, index: usize) -> Expression<'a> {
        Expression::new_computed_member_expression(
            SPAN,
            Expression::new_identifier(SPAN, Str::from_in(name, self.ast.allocator()), &self.ast),
            Expression::new_string_literal(
                SPAN,
                Str::from_in(index.to_string().as_str(), self.ast.allocator()),
                None,
                &self.ast,
            ),
            true,
            &self.ast,
        )
    }

    fn selected_branch(
        &mut self,
        name: &str,
        key: &str,
        branch: &mut Expression<'a>,
    ) -> Expression<'a> {
        let mut captured = Vec::new();
        self.capture_shape(branch, &mut captured);
        let mut reads = BranchReads {
            ast: &self.ast,
            reads: FxHashMap::default(),
        };
        let mut values = oxc_allocator::Vec::new_in(&self.ast);
        values.push(
            Expression::new_string_literal(
                SPAN,
                Str::from_in(key, self.ast.allocator()),
                None,
                &self.ast,
            )
            .into(),
        );
        for (index, (leaf, value)) in captured.into_iter().enumerate() {
            reads
                .reads
                .insert(leaf, self.selected_slot(name, index + 1));
            values.push(value.into());
        }
        reads.visit_expression(branch);
        Expression::new_array_expression(SPAN, values, &self.ast)
    }

    pub(super) fn capture_selection(
        &mut self,
        member: &mut oxc_ast::ast::ComputedMemberExpression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) -> bool {
        let name = self.names.fresh("__devupChoice");
        let mut table = crate::utils::unwrap_syntax_only(&member.object)
            .clone_in_with_semantic_ids(self.ast.allocator());
        match &mut table {
            Expression::ArrayExpression(array) if array.elements.iter().all(|element|!matches!(element,ArrayExpressionElement::SpreadElement(_)))=>{
                let Expression::ArrayExpression(original)=crate::utils::unwrap_syntax_only_mut(&mut member.object)else{return false};
                for(index,(payload,branch))in array.elements.iter_mut().zip(&mut original.elements).enumerate(){
                    if let Some(branch)=branch.as_expression_mut(){
                        *payload=self.selected_branch(&name,&index.to_string(),branch).into();
                    }
                }
            }
            Expression::ObjectExpression(object) if object.properties.iter().all(|property|matches!(property,ObjectPropertyKind::ObjectProperty(property) if property.kind==PropertyKind::Init&&!property.method&&property.key.static_name().is_some()))=>{
                let Expression::ObjectExpression(original)=crate::utils::unwrap_syntax_only_mut(&mut member.object)else{return false};
                for(payload,branch)in object.properties.iter_mut().zip(&mut original.properties){
                    if let(ObjectPropertyKind::ObjectProperty(payload),ObjectPropertyKind::ObjectProperty(branch))=(payload,branch)
                        && let Some(key)=branch.key.static_name(){
                        payload.value=self.selected_branch(&name,&key,&mut branch.value);
                        payload.shorthand=false;
                    }
                }
            }
            _=>return false,
        }
        let key = member.expression.take_in(&self.ast);
        member.expression = self.selected_slot(&name, 0);
        let selection = Expression::new_computed_member_expression(
            member.span,
            table,
            key,
            member.optional,
            &self.ast,
        );
        captured.push((name, selection));
        true
    }
}
