use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::ast::{Expression, Str};
use oxc_span::SPAN;

use super::DevupVisitor;
use crate::ExtractStyleValue;
use crate::extract_style::extract_css::ExtractCss;
use crate::extractor::KeyframesExtractResult;
use crate::extractor::extract_keyframes_from_expression::extract_keyframes_from_expression;
use crate::stylex::StylexFunction;
use crate::stylex::transitions::{IdentityDomain, content_name};
use crate::utils::runtime_value_error;

impl<'a> DevupVisitor<'a> {
    pub(in crate::visit) fn compile_stylex_keyframes(&mut self, expression: &mut Expression<'a>) {
        let Expression::CallExpression(call) = expression else {
            return;
        };
        if !self.is_stylex_call(&call.callee, &StylexFunction::Keyframes) {
            return;
        }
        let [argument] = call.arguments.as_mut_slice() else {
            return;
        };
        let Some(argument @ Expression::ObjectExpression(_)) = argument.as_expression_mut() else {
            return;
        };
        let KeyframesExtractResult {
            keyframes,
            runtime_value,
        } = extract_keyframes_from_expression(&self.ast, argument);
        if let Some(value) = runtime_value {
            self.errors.push((
                call.span.start,
                runtime_value_error("stylex.keyframes", &value),
            ));
        }
        let mut content = String::new();
        for (step, styles) in &keyframes.keyframes {
            content.push_str(step);
            content.push('{');
            for style in styles {
                content.push_str(style.property());
                content.push(':');
                content.push_str(style.value());
                content.push(';');
            }
            content.push('}');
        }
        let name = content_name(&self.filename, IdentityDomain::Keyframes, &content);
        self.styles.insert(ExtractStyleValue::Css(ExtractCss {
            css: format!("@keyframes {name}{{{content}}}"),
            file: self.filename.clone(),
        }));
        self.stylex_pending_keyframe_name = Some(name.clone());
        *expression = Expression::new_string_literal(
            SPAN,
            Str::from_in(&name, self.ast.allocator()),
            None,
            &self.ast,
        );
    }
}
