use super::DevupVisitor;
use crate::{ExtractStyleProp, own_css_plan::Plan, utils::unused_error};
use oxc_ast::ast::{Argument, Expression};

#[path = "own_css_bindings.rs"]
mod bindings;
#[path = "own_css_class.rs"]
mod class;
#[path = "own_css_template.rs"]
mod template;

struct BoundRules<'a> {
    plan: Plan,
    original: Vec<Expression<'a>>,
    slots: Vec<Expression<'a>>,
}

struct RulePart {
    roots: Vec<usize>,
    composing: bool,
}

impl<'a> DevupVisitor<'a> {
    pub(super) fn compile_own_rules(&mut self, expression: &mut Expression<'a>) -> bool {
        let Expression::CallExpression(call) = expression else {
            return false;
        };
        let Some(plan) = self.own_rules.remove(&(call.span.start, call.span.end)) else {
            return false;
        };
        let offset = call.span.start;
        let Some(Expression::ArrayExpression(array)) =
            call.arguments.first().and_then(Argument::as_expression)
        else {
            return false;
        };
        let origin = plan.origin.clone();
        let mut rules = self.bind_own_rules(plan, array);
        let start = self.errors.len();
        *expression = if rules.plan.template.is_some() {
            self.own_template(&mut rules, offset)
        } else {
            let part = RulePart {
                roots: rules.plan.roots.clone(),
                composing: rules.plan.composing,
            };
            self.own_class(&mut rules, part, offset)
        };
        if let Some((location, api)) = origin {
            self.own_failures
                .extend(self.errors.drain(start..).map(|(_, message)| {
                    let message = message.replacen("`css()`", &format!("`{api}()`"), 1);
                    format!("{location}: {message}")
                }));
        }
        true
    }

    fn own_errors(&mut self, offset: u32, styles: &[ExtractStyleProp<'a>]) {
        if let Some((offset, value)) = crate::dead_properties::located_value(styles, offset, false)
        {
            self.errors.push((offset, unused_error("css", &value)));
        }
    }
}
