use super::DevupVisitor;
use crate::finite_styles::FiniteStyles;
use crate::utils::unwrap_syntax_only;
use oxc_ast::ast::{ArrowFunctionBody, Expression};
use oxc_ast_visit::Visit;

struct Returns<'v> {
    values: &'v crate::style_values::StyleValues,
    finite: Vec<FiniteStyles>,
    complete: bool,
}

impl<'a> Visit<'a> for Returns<'_> {
    fn visit_return_statement(&mut self, statement: &oxc_ast::ast::ReturnStatement<'a>) {
        match statement
            .argument
            .as_ref()
            .and_then(|value| self.values.finite(value))
        {
            Some(finite) => self.finite.push(finite.clone()),
            None => self.complete = false,
        }
    }
    fn visit_function(&mut self, _: &oxc_ast::ast::Function<'a>, _: oxc_syntax::scope::ScopeFlags) {
    }
    fn visit_arrow_function_expression(&mut self, _: &oxc_ast::ast::ArrowFunctionExpression<'a>) {}
}

impl<'a> DevupVisitor<'a> {
    pub(super) fn finite_callback(&self, expression: &Expression<'a>) -> Option<FiniteStyles> {
        let mut returns = Returns {
            values: &self.style_values,
            finite: vec![],
            complete: true,
        };
        match unwrap_syntax_only(expression) {
            Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => returns.visit_function_body(body),
                body => return self.style_values.finite(body.as_expression()?).cloned(),
            },
            Expression::FunctionExpression(function)
                if !function.r#async && !function.generator =>
            {
                returns.visit_function_body(function.body.as_ref()?);
            }
            _ => return None,
        }
        if !returns.complete || returns.finite.is_empty() {
            return None;
        }
        let mut results = Vec::new();
        for finite in returns.finite {
            for entry in finite.results {
                if results
                    .iter()
                    .any(|(text, values)| *text == entry.0 && *values != entry.1)
                {
                    return None;
                }
                if !results.contains(&entry) {
                    results.push(entry);
                }
            }
        }
        if !results.iter().any(|(text, _)| text.is_empty()) {
            results.push((String::new(), vec![]));
        }
        Some(FiniteStyles { results })
    }
}
