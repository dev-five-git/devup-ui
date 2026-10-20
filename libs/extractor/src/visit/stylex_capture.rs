use super::{
    AstBuilder, BindingPattern, Expression, FxHashSet, GetAllocator, GetSpan, Program, SPAN,
    Statement, UnaryOperator, VariableDeclarator,
};
use oxc_ast::ast::{AssignmentOperator, AssignmentTarget, VariableDeclarationKind};
use oxc_ast_visit::{Visit, walk};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use std::cell::Cell;

#[derive(Default)]
pub(super) struct Captures {
    names: FxHashSet<String>,
    next: usize,
    scopes: Vec<Option<ScopeId>>,
    eval_scopes: FxHashSet<ScopeId>,
    pub frames: Vec<(bool, Vec<String>)>,
    pub context: Option<&'static str>,
}

impl Captures {
    pub fn reserve(&mut self, program: &Program<'_>) {
        self.visit_program(program);
    }

    pub fn push_frame(&mut self) {
        let visible = self
            .scopes
            .last()
            .is_some_and(|scope| scope.is_some_and(|id| self.eval_scopes.contains(&id)));
        self.frames.push((visible, Vec::new()));
    }

    pub fn capture<'a>(
        &mut self,
        ast: &AstBuilder<'a>,
        test: Expression<'a>,
    ) -> Result<(Expression<'a>, Expression<'a>), (u32, String)> {
        if self.frames.is_empty() || self.context.is_some() {
            return Err((
                test.span().start,
                format!(
                    "StyleX condition capture is not supported in {}; move the value into a variable in a statement body",
                    self.context.unwrap_or("this expression-only context")
                ),
            ));
        }
        if self.frames.last().is_some_and(|(visible, _)| *visible) {
            return Err((
                test.span().start,
                "StyleX condition capture is not supported in a statement body whose bindings are visible to direct eval; move the value into a variable in a scope not visible to direct eval".to_string(),
            ));
        }
        let name = loop {
            let name = format!("_s{}", self.next);
            self.next += 1;
            if self.names.insert(name.clone()) {
                break name;
            }
        };
        if let Some((_, frame)) = self.frames.last_mut() {
            frame.push(name.clone());
        }
        let name = ast.allocator().alloc_str(&name);
        let boolean = Expression::new_unary_expression(
            SPAN,
            UnaryOperator::LogicalNot,
            Expression::new_unary_expression(SPAN, UnaryOperator::LogicalNot, test, ast),
            ast,
        );
        let assignment = Expression::new_assignment_expression(
            SPAN,
            AssignmentOperator::Assign,
            AssignmentTarget::new_assignment_target_identifier(SPAN, name, ast),
            boolean,
            ast,
        );
        Ok((Expression::new_identifier(SPAN, name, ast), assignment))
    }
}

impl<'a> Visit<'a> for Captures {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.push(scope_id.get());
    }
    fn leave_scope(&mut self) {
        self.scopes.pop();
    }
    fn visit_binding_identifier(&mut self, it: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.names.insert(it.name.to_string());
    }
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        self.names.insert(it.name.to_string());
    }
    fn visit_formal_parameters(&mut self, it: &oxc_ast::ast::FormalParameters<'a>) {
        // Parameter-created eval closures cannot see the function body's lexical declarations.
        let scope = self.scopes.pop();
        walk::walk_formal_parameters(self, it);
        self.scopes.extend(scope);
    }
    fn visit_call_expression(&mut self, it: &oxc_ast::ast::CallExpression<'a>) {
        if !it.optional
            && matches!(it.callee.get_inner_expression(), Expression::Identifier(id) if id.name == "eval")
        {
            self.eval_scopes
                .extend(self.scopes.iter().flatten().copied());
        }
        walk::walk_call_expression(self, it);
    }
}

pub(super) fn declaration<'a>(ast: &AstBuilder<'a>, names: Vec<String>) -> Statement<'a> {
    let declarations = names.into_iter().map(|name| {
        VariableDeclarator::new(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, ast.allocator().alloc_str(&name), ast),
            None,
            None,
            false,
            ast,
        )
    });
    Statement::new_variable_declaration(
        SPAN,
        VariableDeclarationKind::Let,
        oxc_allocator::Vec::from_iter_in(declarations, ast),
        false,
        ast,
    )
}
