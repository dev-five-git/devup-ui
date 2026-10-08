//! Semantic consumer reads retain hazard origins before extraction rewrites the AST.

use super::{Change, Changed};
use crate::css_prop::binding_of;
use oxc_ast::ast::{Expression, IdentifierReference, Program};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use rustc_hash::FxHashMap;
use std::rc::Rc;

impl Changed {
    pub(crate) fn consumers(
        &self,
        program: &Program<'_>,
        scoping: Option<&Scoping>,
    ) -> FxHashMap<u32, Vec<Rc<Change>>> {
        let Some(scoping) = scoping.filter(|_| !self.is_empty()) else {
            return FxHashMap::default();
        };
        let mut consumers = Consumers {
            changed: self,
            scoping,
            origins: FxHashMap::default(),
        };
        consumers.visit_program(program);
        consumers.origins
    }
}

struct Consumers<'s> {
    changed: &'s Changed,
    scoping: &'s Scoping,
    origins: FxHashMap<u32, Vec<Rc<Change>>>,
}

impl<'a> Visit<'a> for Consumers<'_> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        let mut reads = Reads {
            changed: self.changed,
            scoping: self.scoping,
            origins: Vec::new(),
        };
        reads.visit_expression(expression);
        self.origins
            .entry(expression.span().start)
            .or_default()
            .extend(reads.origins);
        walk::walk_expression(self, expression);
    }
}

struct Reads<'s> {
    changed: &'s Changed,
    scoping: &'s Scoping,
    origins: Vec<Rc<Change>>,
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if binding_of(self.scoping, identifier).is_some_and(|symbol| {
            self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
        }) {
            let name = identifier.name.as_str();
            self.origins
                .extend(self.changed.whole.get(name).cloned().or_else(|| {
                    self.changed
                        .holding
                        .get(name)
                        .and_then(super::Constant::change)
                }));
        }
    }

    fn visit_ts_type(&mut self, _: &oxc_ast::ast::TSType<'a>) {}
}
