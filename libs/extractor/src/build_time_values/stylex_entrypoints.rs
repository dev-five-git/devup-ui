use oxc_ast::ast::Expression;

use super::{Finder, UTILS, stylex_symbol};

impl Finder<'_, '_> {
    pub(super) fn is_api(&self, callee: &Expression<'_>) -> bool {
        if self
            .stylex
            .function(callee, &|id| stylex_symbol(self.scoping, id))
            .is_some_and(|function| function.requirement().is_some())
        {
            return true;
        }
        match crate::utils::unwrap_syntax_only(callee) {
            Expression::Identifier(_) => self
                .symbol(callee)
                .is_some_and(|symbol| self.apis.contains(&symbol)),
            Expression::StaticMemberExpression(member) => {
                self.symbol(&member.object).is_some_and(|symbol| {
                    self.namespaces.contains(&symbol)
                        && UTILS.contains(&member.property.name.as_str())
                })
            }
            _ => false,
        }
    }
}
