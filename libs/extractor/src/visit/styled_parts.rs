use super::capture::Captured;
use super::{DevupVisitor, Text};
use crate::composition::{Composition, KnownPart};
use crate::extractor::extract_style_from_styled::PreparedStyled;
use crate::styled_reads::Reads;
use crate::utils::unwrap_syntax_only;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Argument, Expression};
use oxc_span::SPAN;

impl<'a> DevupVisitor<'a> {
    pub(super) fn compose_css_template(
        &mut self,
        tag: &mut oxc_ast::ast::TaggedTemplateExpression<'a>,
        captures: &mut Vec<Captured<'a>>,
    ) -> Option<Expression<'a>> {
        self.style_values.read_in_text(&self.ast, &mut tag.quasi);
        let mut literal = Expression::TemplateLiteral(oxc_allocator::Box::new_in(
            tag.quasi.clone_in_with_semantic_ids(self.ast.allocator()),
            &self.ast,
        ));
        if crate::css_utils::literal::lower_with_source(&self.ast, &mut literal, self.source) {
            self.capture_style_argument(&mut literal, captures);
            let mut parts = Vec::new();
            self.known_parts(&literal, &mut parts, Text::Rules)?;
            let (result, known) = self.composed_class(tag.span, parts);
            if let Some(known) = known {
                self.css_styles
                    .insert((tag.span.start, tag.span.end), known);
            }
            return Some(result);
        }
        let segments = crate::css_prop::template_parts(&self.ast, &tag.quasi, true).ok()?;
        let mut parts = Vec::new();
        for mut segment in segments {
            if let Expression::TemplateLiteral(template) = &mut segment {
                for value in &mut template.expressions {
                    self.capture_shape(value, captures);
                }
                self.known_parts(&segment, &mut parts, Text::Rules)?;
            } else {
                self.capture_style_argument(&mut segment, captures);
                self.known_parts(&segment, &mut parts, Text::Classes)?;
            }
        }
        let (result, known) = self.composed_class(tag.span, parts);
        if let Some(known) = known {
            self.css_styles
                .insert((tag.span.start, tag.span.end), known);
        }
        Some(result)
    }
    pub(super) fn prepare_styled_template(&self, expression: &mut Expression<'a>) {
        if let Expression::TaggedTemplateExpression(tag) = expression {
            if matches!(unwrap_syntax_only(&tag.tag), Expression::CallExpression(factory) if factory.arguments.len() != 1)
            {
                return;
            }
            let mut literal = Expression::TemplateLiteral(oxc_allocator::Box::new_in(
                tag.quasi.clone_in_with_semantic_ids(self.ast.allocator()),
                &self.ast,
            ));
            let segments = if crate::css_utils::literal::lower_with_source(
                &self.ast,
                &mut literal,
                self.source,
            ) {
                vec![literal]
            } else if let Ok(segments) =
                crate::css_prop::styled_template_parts(&self.ast, &tag.quasi, &self.style_values)
            {
                segments
            } else {
                return;
            };
            *expression = Expression::new_call_expression(
                tag.span,
                tag.tag.clone_in_with_semantic_ids(self.ast.allocator()),
                None::<oxc_allocator::Box<'a, oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
                oxc_allocator::Vec::from_iter_in(
                    segments.into_iter().map(Argument::from),
                    &self.ast,
                ),
                false,
                &self.ast,
            );
        }
    }
    pub(super) fn prepare_styled_parts(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
        attrs: &mut [Expression<'a>],
    ) -> Option<PreparedStyled<'a>> {
        let source = expression.clone_in_with_semantic_ids(self.ast.allocator());
        let original_attrs: Vec<_> = attrs
            .iter()
            .map(|attr| attr.clone_in_with_semantic_ids(self.ast.allocator()))
            .collect();
        let captured = captures.len();
        let prepared = self.prepare_styled_inner(expression, captures, attrs);
        if prepared.is_none() {
            *expression = source;
            for (attr, original) in attrs.iter_mut().zip(original_attrs) {
                *attr = original;
            }
            captures.truncate(captured);
        }
        prepared
    }

    fn prepare_styled_inner(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
        attrs: &mut [Expression<'a>],
    ) -> Option<PreparedStyled<'a>> {
        let Expression::CallExpression(call) = expression else {
            return None;
        };
        let direct = matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(_));
        let index = usize::from(direct && call.arguments.len() == 2);
        if direct
            && (call.arguments.len() != 2
                || !call.arguments[1].as_expression().is_some_and(|value| {
                    matches!(
                        unwrap_syntax_only(value),
                        Expression::ObjectExpression(_)
                            | Expression::ConditionalExpression(_)
                            | Expression::LogicalExpression(_)
                            | Expression::ArrowFunctionExpression(_)
                            | Expression::FunctionExpression(_)
                    ) || crate::css_utils::literal::is_rule_text(value)
                }))
        {
            return None;
        }
        let mut reads = Reads::default();
        for attr in attrs.iter() {
            reads.read_in(attr);
        }
        for argument in call.arguments.iter().skip(index) {
            if let Some(value) = argument.as_expression() {
                reads.read_in(value);
            }
        }
        for attr in attrs {
            let name = self.names.fresh("__devupAttrs");
            captures.push(self.capture_as(name, attr));
        }
        let mut parts = Vec::new();
        let mut renders = Vec::new();
        for argument in call.arguments.iter_mut().skip(index) {
            let value = argument.as_expression_mut()?;
            self.style_values.read_keys(&self.ast, value);
            let callback = matches!(
                unwrap_syntax_only(value),
                Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
            );
            let text = match unwrap_syntax_only(value) {
                Expression::TemplateLiteral(_) => Text::Rules,
                Expression::StringLiteral(literal) if literal.value.contains(':') => Text::Rules,
                _ => Text::Classes,
            };
            if !callback && self.known_parts(value, &mut Vec::new(), text).is_none() {
                return None;
            }
            let render = if matches!(
                unwrap_syntax_only(value),
                Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
            ) {
                Some(self.prepare_callback(value, captures)?)
            } else {
                crate::css_utils::literal::lower_with_source(&self.ast, value, self.source);
                if self.capture_literal_styled(value, captures, &mut renders) {
                    self.check_style_orders(value, false);
                } else if let Expression::TemplateLiteral(template) = value {
                    for interpolation in &mut template.expressions {
                        self.capture_shape(interpolation, captures);
                    }
                } else {
                    self.capture_style_argument(value, captures);
                }
                None
            };
            if let Some((mut callback_parts, invocation)) = render {
                parts.append(&mut callback_parts);
                renders.push(invocation);
            } else {
                let text = match unwrap_syntax_only(value) {
                    Expression::TemplateLiteral(_) => Text::Rules,
                    Expression::StringLiteral(literal) if literal.value.contains(':') => {
                        Text::Rules
                    }
                    _ => Text::Classes,
                };
                self.known_parts(value, &mut parts, text)?;
            }
        }
        let (styles, classes) = self.ordered_styled_parts(parts);
        call.arguments.truncate(index);
        call.arguments
            .push(Argument::from(Expression::new_object_expression(
                SPAN,
                oxc_allocator::Vec::new_in(&self.ast),
                &self.ast,
            )));
        Some(PreparedStyled {
            styles,
            classes,
            reads,
            renders,
        })
    }

    fn ordered_styled_parts(
        &mut self,
        parts: Vec<KnownPart<'a>>,
    ) -> (Vec<crate::ExtractStyleProp<'a>>, Vec<Expression<'a>>) {
        let mut composition = Composition::default();
        let mut classes = Vec::new();
        for part in parts {
            match part {
                KnownPart::Styles(styles) => {
                    let props = self.part_props(0, styles, Some("styled"));
                    composition.apply(&self.ast, props);
                }
                KnownPart::Conditional {
                    test,
                    consequent,
                    alternate,
                } => {
                    let yes = self.part_props(0, consequent, Some("styled"));
                    let no = self.part_props(0, alternate, Some("styled"));
                    composition.apply_conditional(&self.ast, &test, yes, no);
                }
                KnownPart::Class(class) => classes.push(class),
            }
        }
        (composition.into_props(), classes)
    }
}
