use super::{DevupVisitor, Expression, call_with_values};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::IdentifierReference;
use oxc_ast_visit::{Visit, VisitMut, walk_mut};

struct TestReads<'n> {
    name: &'n str,
    count: usize,
}

impl<'a> Visit<'a> for TestReads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if identifier.name == self.name {
            self.count += 1;
        }
    }
}

struct InlineTest<'v, 'a> {
    visitor: &'v DevupVisitor<'a>,
    name: &'v str,
    value: &'v Expression<'a>,
}

impl<'a> VisitMut<'a> for InlineTest<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if matches!(expression, Expression::Identifier(identifier) if identifier.name == self.name)
        {
            *expression = self
                .value
                .clone_in_with_semantic_ids(self.visitor.ast.allocator());
        } else {
            walk_mut::walk_expression(self, expression);
        }
    }
}

impl<'a> DevupVisitor<'a> {
    pub(super) fn bind_stylex_namespace_alias(
        &mut self,
        symbol: oxc_syntax::symbol::SymbolId,
        from: &IdentifierReference<'a>,
    ) {
        let offset = from.span.start;
        if let Some(source) = self.bindings.symbol(from) {
            if self.stylex_namespaces.contains_key(&source)
                && (!self.bindings.unchanged(symbol) || !self.bindings.unchanged(source))
            {
                self.errors.push((
                    offset,
                    crate::utils::build_time_error(
                        "stylex namespace alias",
                        from.name.as_str(),
                        "a reassigned namespace/alias or written member makes its metadata unknowable; use unchanged namespace bindings and members instead of reassigning them",
                    ),
                ));
                return;
            }
            if let Some(namespace) = self.stylex_namespaces.get(&source).cloned() {
                self.stylex_namespaces.insert(symbol, namespace);
            }
            if let Some(keys) = self.stylex_keys.get(&source).cloned() {
                self.stylex_keys.insert(symbol, keys);
            }
        }
    }

    pub(super) fn capture_stylex_conditions(
        &self,
        values: Vec<(String, Expression<'a>)>,
        mut body: Expression<'a>,
    ) -> Expression<'a> {
        if values.is_empty() {
            return body;
        }
        if let [(name, value)] = values.as_slice() {
            let mut reads = TestReads { name, count: 0 };
            reads.visit_expression(&body);
            if reads.count == 1 {
                InlineTest {
                    visitor: self,
                    name,
                    value,
                }
                .visit_expression(&mut body);
                return body;
            }
        }
        call_with_values(&self.ast, values, body)
    }
}
