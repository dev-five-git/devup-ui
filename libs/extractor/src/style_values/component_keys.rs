use super::{Reads, StyleValues};
use oxc_ast::ast::{Expression, PropertyKey};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::VisitMut;

struct Keys<'s, 'a> {
    reads: Reads<'s, 'a>,
}

impl<'a> VisitMut<'a> for Keys<'_, 'a> {
    fn visit_property_key(&mut self, key: &mut PropertyKey<'a>) {
        self.reads.visit_property_key(key);
    }
}

impl StyleValues {
    pub(crate) fn read_keys<'a>(&self, ast: &AstBuilder<'a>, expression: &mut Expression<'a>) {
        if let Some(reads) = self.reads(ast) {
            Keys { reads }.visit_expression(expression);
        }
    }
}
