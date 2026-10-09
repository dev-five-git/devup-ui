use oxc_ast::{AstKind, ast::IdentifierReference};
use oxc_ast_visit::Visit;
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};

use super::{Closed, binding_of};

#[cfg(test)]
mod tests;

pub(super) fn closed<'a>(
    kind: AstKind<'a>,
    proof: &mut Closed<'_>,
    semantic: &Semantic<'a>,
) -> bool {
    let mut callable = Callable {
        proof,
        semantic,
        span: kind.span(),
    };
    match kind {
        AstKind::Function(function) => {
            callable.visit_formal_parameters(&function.params);
            let Some(body) = &function.body else {
                return false;
            };
            callable.visit_function_body(body);
        }
        AstKind::ArrowFunctionExpression(function) => {
            callable.visit_formal_parameters(&function.params);
            callable.visit_arrow_function_body(&function.body);
        }
        _ => return false,
    }
    callable.proof.exact
}

struct Callable<'p, 's, 'a> {
    proof: &'p mut Closed<'s>,
    semantic: &'p Semantic<'a>,
    span: Span,
}

impl<'a> Visit<'a> for Callable<'_, '_, 'a> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if binding_of(self.semantic.scoping(), identifier).is_some_and(|symbol| {
            self.span
                .contains_inclusive(self.semantic.scoping().symbol_span(symbol))
        }) {
            return;
        }
        self.proof.visit_identifier_reference(identifier);
    }
}
