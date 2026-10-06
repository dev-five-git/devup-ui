use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_span::Span;

use crate::module_loader::{Mapped, demand::Demand};
use crate::utils::unwrap_syntax_only;

pub(super) struct Properties {
    span: Span,
    kept: Vec<(Span, Option<Self>)>,
    pub omitted: Vec<Span>,
}

impl Properties {
    pub fn select(expression: &Expression<'_>, demand: &Demand) -> Option<Self> {
        if demand.whole {
            return None;
        }
        let Expression::ObjectExpression(object) = unwrap_syntax_only(expression) else {
            return None;
        };
        if object.properties.iter().any(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => {
                property.computed || property.key.static_name().is_none()
            }
            ObjectPropertyKind::SpreadProperty(_) => true,
        }) {
            return None;
        }
        let mut kept = Vec::new();
        let mut omitted = Vec::new();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            let Some(name) = property.key.static_name() else {
                continue;
            };
            match demand.child(&name) {
                Some(child) => {
                    let nested = Self::select(&property.value, child);
                    if let Some(nested) = &nested {
                        omitted.extend_from_slice(&nested.omitted);
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
        })
    }

    pub fn write(&self, source: &str, mapped: &mut Mapped) {
        mapped.synthesize(self.span.start, "{");
        for (span, nested) in &self.kept {
            if let Some(nested) = nested {
                mapped.copy(source, Span::new(span.start, nested.span.start));
                nested.write(source, mapped);
                mapped.copy(source, Span::new(nested.span.end, span.end));
            } else {
                mapped.copy(source, *span);
            }
            mapped.synthesize(span.end, ",");
        }
        mapped.synthesize(self.span.end, "}");
    }

    pub fn replace(&self, source: &str, span: Span, mapped: &mut Mapped) {
        mapped.copy(source, Span::new(span.start, self.span.start));
        self.write(source, mapped);
        mapped.copy(source, Span::new(self.span.end, span.end));
    }
}
