use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::{GetSpan, SPAN};

use crate::utils::{call_with_values, is_same_expression};

/// Ordered reads shared by the class and variable expressions of one render.
pub(super) struct Evaluations<'b, 'a> {
    pub builder: &'b AstBuilder<'a>,
    source: String,
    pub bindings: Vec<(String, Expression<'a>)>,
    next: usize,
}

impl<'b, 'a> Evaluations<'b, 'a> {
    pub const fn new(builder: &'b AstBuilder<'a>, source: String) -> Self {
        Self {
            builder,
            source,
            bindings: Vec::new(),
            next: 0,
        }
    }

    pub fn capture(&mut self, value: &Expression<'a>, guard: &Expression<'a>) -> Expression<'a> {
        let name = loop {
            let name = format!("__devupRecord{}", self.next);
            self.next += 1;
            if !self.source.contains(&name) {
                break name;
            }
        };
        let value = self.choose(guard, value, &self.undefined());
        self.bindings.push((name.clone(), value));
        Expression::new_identifier(
            SPAN,
            oxc_ast::ast::Ident::from_in(name.as_str(), self.builder.allocator()),
            self.builder,
        )
    }

    pub fn boolean(&self, value: bool) -> Expression<'a> {
        Expression::new_boolean_literal(SPAN, value, self.builder)
    }

    pub fn undefined(&self) -> Expression<'a> {
        Expression::new_identifier(SPAN, "undefined", self.builder)
    }

    pub fn choose(
        &self,
        test: &Expression<'a>,
        yes: &Expression<'a>,
        no: &Expression<'a>,
    ) -> Expression<'a> {
        let clone = |value: &Expression<'a>| value.clone_in(self.builder.allocator());
        match (test, yes, no) {
            (Expression::BooleanLiteral(value), _, _) => clone(if value.value { yes } else { no }),
            (_, Expression::BooleanLiteral(yes), Expression::BooleanLiteral(no))
                if yes.value == no.value =>
            {
                self.boolean(yes.value)
            }
            _ if is_same_expression(yes, no) => clone(yes),
            _ => Expression::new_conditional_expression(
                test.span(),
                clone(test),
                clone(yes),
                clone(no),
                self.builder,
            ),
        }
    }
}

pub(super) fn bind_render<'a>(
    builder: &AstBuilder<'a>,
    bindings: Vec<(String, Expression<'a>)>,
    mut body: Expression<'a>,
) -> Expression<'a> {
    // Dependent arguments must be inside the scope of the preceding read.
    for binding in bindings.into_iter().rev() {
        body = call_with_values(builder, vec![binding], body);
    }
    body
}

#[cfg(test)]
mod tests {
    use boa_engine::{Context, Source};
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    use super::*;
    use crate::utils::readable_code;

    #[rstest]
    #[case::equal_true(true, true)]
    #[case::equal_false(false, false)]
    #[case::different_true(true, false)]
    #[case::different_false(false, true)]
    fn boolean_choice_when_test_is_captured(
        #[case] yes: bool,
        #[case] no: bool,
        #[values(false, true)] on: bool,
    ) {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let source = "(trace.push('test'), on);";
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("test fixture must be an expression");
        };
        let mut reads = Evaluations::new(&builder, source.to_string());
        let test = reads.capture(&statement.expression, &reads.boolean(true));
        let choice = reads.choose(&test, &reads.boolean(yes), &reads.boolean(no));
        match &choice {
            Expression::BooleanLiteral(value) => {
                assert_eq!(yes, no);
                assert_eq!(value.value, yes);
            }
            Expression::ConditionalExpression(_) => assert_ne!(yes, no),
            _ => panic!("Boolean choice must be constant or conditional"),
        }
        let render = bind_render(&builder, reads.bindings, choice);
        let script = format!(
            "const trace = []; const on = {on}; const result = {}; JSON.stringify([result, trace]);",
            readable_code(&render)
        );
        let mut context = Context::default();
        let result = match context.eval(Source::from_bytes(script.as_bytes())) {
            Ok(result) => result,
            Err(error) => panic!("Boolean choice evaluation failed: {error}"),
        };
        let result = match result.as_string() {
            Some(result) => result.to_std_string_escaped(),
            None => panic!("Boolean choice must return a JSON string"),
        };
        assert_eq!(
            result,
            format!(r#"[{},["test"]]"#, if on { yes } else { no })
        );
    }
}
