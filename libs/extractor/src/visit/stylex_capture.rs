use super::{AstBuilder, Expression, FxHashSet, GetSpan, Program, SPAN, UnaryOperator};
use oxc_ast_visit::{Visit, walk};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use std::cell::Cell;

#[derive(Default)]
pub(super) struct Captures {
    names: FxHashSet<String>,
    next: usize,
    scopes: Vec<Option<ScopeId>>,
    eval_scopes: FxHashSet<ScopeId>,
    /// Whether each statement list open is visible to direct eval
    pub frames: Vec<bool>,
    pub context: Option<&'static str>,
}

impl Captures {
    /// Generated props share the existing scope and direct-eval admission gates.
    pub(super) fn binds(&self) -> bool {
        self.context.is_none() && self.frames.last().is_some_and(|visible| !*visible)
    }

    pub fn reserve(&mut self, program: &Program<'_>) {
        self.visit_program(program);
    }

    pub fn push_frame(&mut self) {
        let visible = self
            .scopes
            .last()
            .is_some_and(|scope| scope.is_some_and(|id| self.eval_scopes.contains(&id)));
        self.frames.push(visible);
    }

    /// A fresh name for the result of `test`, and the boolean it holds
    pub fn capture<'a>(
        &mut self,
        ast: &AstBuilder<'a>,
        test: Expression<'a>,
    ) -> Result<(String, Expression<'a>), (u32, String)> {
        if self.frames.is_empty() || self.context.is_some() {
            return Err((
                test.span().start,
                format!(
                    "StyleX condition capture is not supported in {}; move the value into a variable in a statement body",
                    self.context.unwrap_or("this expression-only context")
                ),
            ));
        }
        if self.frames.last().is_some_and(|visible| *visible) {
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
        let boolean = Expression::new_unary_expression(
            SPAN,
            UnaryOperator::LogicalNot,
            Expression::new_unary_expression(SPAN, UnaryOperator::LogicalNot, test, ast),
            ast,
        );
        Ok((name, boolean))
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

/// The `parts` evaluated in order, giving the last
pub(super) fn sequence<'a>(
    ast: &AstBuilder<'a>,
    parts: &mut Vec<Expression<'a>>,
) -> Expression<'a> {
    if parts.len() == 1 {
        return parts.remove(0);
    }
    Expression::new_sequence_expression(
        SPAN,
        oxc_allocator::Vec::from_iter_in(std::mem::take(parts), ast),
        ast,
    )
}

/// The tests a `stylex.props()` call runs, in order, each with the name its
/// result is read by when it is read
pub(super) type Evaluations<'a> = Vec<(Option<String>, Expression<'a>)>;
