use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{Expression, Str, TemplateElement, TemplateElementValue, TemplateLiteral};
use oxc_span::SPAN;

use super::ClassEvaluation;
use crate::{assignment_lowering::projection, utils::call_with_values};

impl<'a> ClassEvaluation<'_, 'a> {
    pub(super) fn template(
        &self,
        source: &TemplateLiteral<'a>,
        variants: [&TemplateLiteral<'a>; 2],
    ) -> Expression<'a> {
        let ast = self.0;
        let [primary, alternate] = variants;
        let mut bindings = Vec::with_capacity(source.expressions.len());
        let mut values = Vec::with_capacity(source.expressions.len());
        for (index, ((source, primary), alternate)) in source
            .expressions
            .iter()
            .zip(&primary.expressions)
            .zip(&alternate.expressions)
            .enumerate()
        {
            let name = format!("__devupClassPart{index}");
            values.push((
                name.clone(),
                self.coerced(self.lower(source, [primary, alternate])),
            ));
            bindings.push(name);
        }
        let prepared = |template: &TemplateLiteral<'a>, slot| {
            Expression::new_template_literal(
                template.span,
                template.quasis.clone_in(ast.allocator()),
                oxc_allocator::Vec::from_iter_in(
                    bindings.iter().map(|name| projection(ast, name, slot)),
                    ast,
                ),
                ast,
            )
        };
        call_with_values(
            ast,
            values,
            self.tuple(
                [
                    prepared(primary, 0),
                    prepared(source, 2),
                    prepared(alternate, 3),
                ],
                false,
            ),
        )
    }

    fn coerced(&self, value: Expression<'a>) -> Expression<'a> {
        let ast = self.0;
        let string = Expression::new_identifier(SPAN, "__devupClassString", ast);
        let coerced = call_with_values(
            ast,
            vec![(
                "__devupClassString".to_string(),
                Expression::new_template_literal(
                    SPAN,
                    oxc_allocator::Vec::from_array_in(
                        [
                            TemplateElement::new(
                                SPAN,
                                TemplateElementValue {
                                    raw: Str::from_in("", ast.allocator()),
                                    cooked: None,
                                },
                                false,
                                ast,
                            ),
                            TemplateElement::new(
                                SPAN,
                                TemplateElementValue {
                                    raw: Str::from_in("", ast.allocator()),
                                    cooked: None,
                                },
                                true,
                                ast,
                            ),
                        ],
                        ast,
                    ),
                    oxc_allocator::Vec::from_array_in(
                        [projection(ast, "__devupClassPart", 2)],
                        ast,
                    ),
                    ast,
                ),
            )],
            self.tuple(
                [
                    string.clone_in(ast.allocator()),
                    string.clone_in(ast.allocator()),
                    string,
                ],
                false,
            ),
        );
        call_with_values(
            ast,
            vec![("__devupClassPart".to_string(), value)],
            Expression::new_conditional_expression(
                SPAN,
                projection(ast, "__devupClassPart", 1),
                coerced,
                Expression::new_identifier(SPAN, "__devupClassPart", ast),
                ast,
            ),
        )
    }
}
