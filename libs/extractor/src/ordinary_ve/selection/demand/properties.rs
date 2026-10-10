use oxc_ast::ast::{Expression, ObjectProperty, ObjectPropertyKind, PropertyKind};
use oxc_semantic::Semantic;
use oxc_span::Span;

use crate::module_loader::{Mapped, demand::Demand};
use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};

pub(super) struct Properties {
    span: Span,
    kept: Vec<(Span, Option<Self>)>,
    pub omitted: Vec<Span>,
    forwarded: Vec<(Span, Demand)>,
}

impl Properties {
    pub fn select(
        expression: &Expression<'_>,
        demand: &Demand,
        semantic: &Semantic<'_>,
    ) -> Option<Self> {
        if demand.whole {
            return None;
        }
        let Expression::ObjectExpression(object) = unwrap_syntax_only(expression) else {
            return None;
        };
        if object.properties.iter().any(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => name(property, semantic).is_none(),
            ObjectPropertyKind::SpreadProperty(_) => true,
        }) {
            return None;
        }
        let mut inherited = demand.clone();
        for property in &object.properties {
            if let ObjectPropertyKind::ObjectProperty(property) = property
                && !prototype(property)
                && let Some(name) = name(property, semantic)
            {
                inherited.members.remove(&name);
            }
        }
        let mut kept = Vec::new();
        let mut omitted = Vec::new();
        let mut forwarded = Vec::new();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            let Some(name) = name(property, semantic) else {
                continue;
            };
            let child = if prototype(property) {
                (!inherited.members.is_empty()).then_some(&inherited)
            } else {
                demand.child(&name)
            };
            match child {
                Some(child) => {
                    let nested = Self::select(&property.value, child, semantic);
                    if let Some(nested) = &nested {
                        omitted.extend_from_slice(&nested.omitted);
                        forwarded.extend_from_slice(&nested.forwarded);
                    } else if let Some(dependency) = forward(&property.value, child) {
                        forwarded.push(dependency);
                    }
                    kept.push((property.span, nested));
                }
                None => omitted.push(property.span),
            }
        }
        Some(Self {
            span: object.span,
            kept,
            omitted,
            forwarded,
        })
    }

    pub fn dependency(&self, span: Span) -> Option<&Demand> {
        self.forwarded
            .iter()
            .find_map(|(read, demand)| (*read == span).then_some(demand))
    }

    pub fn write(
        &self,
        source: crate::ordinary_ve::execution::SelectedModule<'_>,
        mapped: &mut Mapped,
    ) {
        mapped.synthesize(self.span.start, "{");
        for (span, nested) in &self.kept {
            if let Some(nested) = nested {
                crate::ordinary_ve::execution::imports::copy(
                    source,
                    Span::new(span.start, nested.span.start),
                    mapped,
                );
                nested.write(source, mapped);
                crate::ordinary_ve::execution::imports::copy(
                    source,
                    Span::new(nested.span.end, span.end),
                    mapped,
                );
            } else {
                crate::ordinary_ve::execution::imports::copy(source, *span, mapped);
            }
            mapped.synthesize(span.end, ",");
        }
        mapped.synthesize(self.span.end, "}");
    }

    pub fn replace(
        &self,
        source: crate::ordinary_ve::execution::SelectedModule<'_>,
        span: Span,
        mapped: &mut Mapped,
    ) {
        crate::ordinary_ve::execution::imports::copy(
            source,
            Span::new(span.start, self.span.start),
            mapped,
        );
        self.write(source, mapped);
        crate::ordinary_ve::execution::imports::copy(
            source,
            Span::new(self.span.end, span.end),
            mapped,
        );
    }
}

fn name(property: &ObjectProperty<'_>, semantic: &Semantic<'_>) -> Option<String> {
    if property.computed {
        super::super::static_key::resolve(property.key.as_expression()?, semantic)
    } else {
        property.key.static_name().map(std::borrow::Cow::into_owned)
    }
}

fn prototype(property: &ObjectProperty<'_>) -> bool {
    !property.computed
        && !property.method
        && !property.shorthand
        && property.kind == PropertyKind::Init
        && property
            .key
            .static_name()
            .is_some_and(|name| name == "__proto__")
}

pub(super) fn forward(expression: &Expression<'_>, demand: &Demand) -> Option<(Span, Demand)> {
    match unwrap_syntax_only(expression) {
        Expression::Identifier(identifier) => Some((identifier.span, demand.clone())),
        Expression::StaticMemberExpression(member) => forward(
            &member.object,
            &Demand::prefixed(member.property.name.as_str(), demand),
        ),
        Expression::ComputedMemberExpression(member) => {
            let key = get_string_by_literal_expression(unwrap_syntax_only(&member.expression))?;
            forward(&member.object, &Demand::prefixed(&key, demand))
        }
        _ => None,
    }
}
