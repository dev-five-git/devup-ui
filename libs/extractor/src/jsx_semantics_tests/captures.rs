use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, Expression};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Default)]
struct Captures {
    wrappers: Vec<(Vec<String>, Vec<String>)>,
}

impl<'a> Visit<'a> for Captures {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::ArrowFunctionExpression(arrow) =
            crate::utils::unwrap_syntax_only(&call.callee)
        {
            let names: Vec<_> = arrow
                .params
                .items
                .iter()
                .filter_map(|parameter| {
                    parameter
                        .pattern
                        .get_identifier_name()
                        .map(|name| name.to_string())
                })
                .collect();
            if names.iter().any(|name| {
                name.starts_with("__devupSpread")
                    || name.starts_with("__devupValue")
                    || name.starts_with("__devupBranch")
            }) {
                let values = call
                    .arguments
                    .iter()
                    .map(crate::utils::readable_argument)
                    .collect();
                self.wrappers.push((names, values));
            }
        }
        walk::walk_call_expression(self, call);
    }
}

pub(super) fn materialized(identifier: &str, code: &str) -> String {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    let mut captures = Captures::default();
    captures.visit_program(&parsed.program);
    captures
        .wrappers
        .iter()
        .rev()
        .fold(identifier.to_string(), |expression, (names, values)| {
            if names.iter().any(|name| expression.contains(name)) {
                format!(
                    "(({}) => ({}))({})",
                    names.join(","),
                    expression,
                    values.join(",")
                )
            } else {
                expression
            }
        })
}
