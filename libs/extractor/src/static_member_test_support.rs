use oxc_ast::ast::Expression;
use oxc_ast_visit::{Visit, walk};

pub(super) fn contains(code: &str, path: &[&str]) -> bool {
    struct Paths<'p> {
        expected: &'p [&'p str],
        found: bool,
    }
    fn components(expression: &Expression<'_>) -> Option<Vec<String>> {
        let (object, key) = match expression {
            Expression::Identifier(identifier) => return Some(vec![identifier.name.to_string()]),
            Expression::StaticMemberExpression(member) => {
                (&member.object, member.property.name.to_string())
            }
            Expression::ComputedMemberExpression(member) => (
                &member.object,
                member
                    .static_property_name()
                    .and_then(|key| key.as_str().map(str::to_string))
                    .or_else(|| {
                        crate::utils::get_string_by_literal_expression(&member.expression)
                            .map(std::borrow::Cow::into_owned)
                    })?,
            ),
            _ => return None,
        };
        let mut path = components(object)?;
        path.push(key);
        Some(path)
    }
    impl<'a> Visit<'a> for Paths<'_> {
        fn visit_expression(&mut self, expression: &Expression<'a>) {
            if components(expression).is_some_and(|path| path == self.expected) {
                self.found = true;
            }
            walk::walk_expression(self, expression);
        }
    }
    let arena = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&arena, code, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut paths = Paths {
        expected: path,
        found: false,
    };
    paths.visit_program(&parsed.program);
    paths.found
}
