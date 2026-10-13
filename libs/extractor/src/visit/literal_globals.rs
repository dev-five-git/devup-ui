use super::DevupVisitor;
use crate::{
    ExtractStyleProp,
    utils::{fixed_value, runtime_value_error},
};
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;

impl<'a> DevupVisitor<'a> {
    pub(super) fn literal_global(&mut self, expression: &mut Expression<'a>) -> bool {
        let Some(text) =
            crate::css_utils::literal::CssText::with_source(&self.ast, expression, self.source)
        else {
            return false;
        };
        let Some(props) = crate::css_utils::global::extract_text(&self.ast, &text, &self.filename)
        else {
            return false;
        };
        self.error_disposition
            .include(crate::style_diagnostics::collect(&props, &mut self.errors));
        if let Some(runtime) = fixed_value(&props) {
            self.errors.push((
                expression.span().start,
                runtime_value_error("globalCss", &runtime),
            ));
        }
        self.styles.extend(
            props
                .into_iter()
                .flat_map(ExtractStyleProp::into_extract)
                .map(|mut style| {
                    style.set_style_order(0);
                    style
                }),
        );
        *expression = Expression::new_object_expression(
            expression.span(),
            oxc_allocator::Vec::new_in(&self.ast),
            &self.ast,
        );
        true
    }
}
