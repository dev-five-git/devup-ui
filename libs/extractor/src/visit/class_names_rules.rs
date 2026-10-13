use super::class_names_parts::{LocalOutput, LocalSource};
use super::{
    AstBuilder, BinaryOperator, CloneIn, DevupVisitor, Expression, ExtractResult, ExtractStyleProp,
    FromIn, GetAllocator, GetSpan, KnownStyles, LiteralHandling, ObjectPropertyKind, SPAN, Str,
    Text, runtime_value, runtime_value_error, set_prop_order, unplaced_error,
};
use crate::extractor::extract_style_from_expression::extract_rule_styles;
use crate::gen_class_name::roots::ClassPayload;

mod finite;
mod literal;

pub(super) struct LocalRules<'v, 's, 'a, S: LocalSource<'a>> {
    visitor: &'v mut DevupVisitor<'a>,
    source: &'s S,
}

impl<'v, 's, 'a, S: LocalSource<'a>> LocalRules<'v, 's, 'a, S> {
    pub(super) const fn new(visitor: &'v mut DevupVisitor<'a>, source: &'s S) -> Self {
        Self { visitor, source }
    }

    pub(super) fn part_props_local(
        &mut self,
        offset: u32,
        styles: Vec<KnownStyles<'a>>,
    ) -> Vec<ExtractStyleProp<'a, S::Class>> {
        let mut props: Vec<ExtractStyleProp<'a, S::Class>> = Vec::new();
        for styles in styles {
            match styles {
                KnownStyles::Known(values) => {
                    props.extend(values.into_iter().map(ExtractStyleProp::Static));
                }
                KnownStyles::Finite(finite, saved) => {
                    props.extend(finite::project(&self.visitor.ast, &finite, &saved));
                }
                KnownStyles::Rules(mut rules) => {
                    crate::css_utils::literal::lower_with_source(
                        &self.visitor.ast,
                        &mut rules,
                        self.visitor.source,
                    );
                    if let Some(styles) = self.visitor.literal_scope_local(&rules, self.source) {
                        props.extend(styles);
                        continue;
                    }
                    self.visitor
                        .style_values
                        .read_in(&self.visitor.ast, &mut rules);
                    let ExtractResult {
                        styles,
                        style_order,
                        ..
                    } = extract_rule_styles(
                        &self.visitor.ast,
                        None,
                        &mut rules,
                        0,
                        &None,
                        LiteralHandling::ExpandResponsiveThemeToken,
                    );
                    let mut styles = styles
                        .into_iter()
                        .map(|prop| prop.map_payload(S::Class::from_rule))
                        .collect::<Vec<_>>();
                    let observation: Vec<ExtractStyleProp<'a>> = styles
                        .iter()
                        .map(|prop| {
                            prop.clone_payload_in(
                                self.visitor.ast.allocator(),
                                ClassPayload::clone_expression,
                            )
                        })
                        .collect();
                    self.visitor
                        .error_disposition
                        .include(crate::style_diagnostics::collect(
                            &observation,
                            &mut self.visitor.errors,
                        ));
                    if let Some(value) = runtime_value(&observation) {
                        self.visitor
                            .errors
                            .push((offset, runtime_value_error("css", &value)));
                    }
                    if let Some(order) = style_order {
                        for prop in &mut styles {
                            set_prop_order(prop, order);
                        }
                    }
                    props.extend(styles);
                }
            }
        }
        props
    }
}
