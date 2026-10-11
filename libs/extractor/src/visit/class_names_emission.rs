use super::capture::Captured;
use super::class_names_parts::{
    CapturedSource, LocalClass, LocalKnownPart, LocalParts, LocalSource, UncapturedSource,
};
use super::class_names_rules::LocalRules;
use super::{
    CLASS_NAMES_PART, DevupVisitor, Expression, ExtractStyleProp, ExtractStyleValue, GetAllocator,
    SPAN, StringLiteral, Text, element_error, readable_code,
};
use crate::composition::Composition;
use crate::gen_class_name::emission::{ClassPlacement, gen_normalised, merge_roots};
use crate::gen_class_name::roots::{
    CapturedClassBody, ClassConstructors, ClassPayload, FinishedClass,
};

/// Only the unchanged capture pass supplies the owned first element and ordered remainder.
pub(super) struct NonemptyCaptures<'a> {
    first: Captured<'a>,
    rest: Vec<Captured<'a>>,
}

impl<'a> NonemptyCaptures<'a> {
    fn from_actual(captures: Vec<Captured<'a>>) -> Option<Self> {
        let mut captures = captures.into_iter();
        let first = captures.next()?;
        Some(Self {
            first,
            rest: captures.collect(),
        })
    }

    fn into_values(self) -> Vec<Captured<'a>> {
        std::iter::once(self.first).chain(self.rest).collect()
    }
}

enum PreparedClassNames<'a> {
    Uncaptured {
        parts: Option<Vec<LocalKnownPart<'a, FinishedClass<'a>>>>,
    },
    Captured {
        captures: NonemptyCaptures<'a>,
        parts: Option<Vec<LocalKnownPart<'a, CapturedClassBody<'a>>>>,
    },
}

impl<'a> DevupVisitor<'a> {
    pub(super) fn finish_class_names_local(
        &mut self,
        value: &Expression<'a>,
        captures: Vec<Captured<'a>>,
        span: oxc_span::Span,
    ) -> FinishedClass<'a> {
        let prepared = match NonemptyCaptures::from_actual(captures) {
            None => PreparedClassNames::Uncaptured {
                parts: collect(self, value, &UncapturedSource),
            },
            Some(captures) => {
                let source: CapturedSource<'_, 'a> = &captures;
                let parts = collect(self, value, &source);
                PreparedClassNames::Captured { captures, parts }
            }
        };
        match prepared {
            PreparedClassNames::Uncaptured { parts } => match parts {
                Some(parts) => self.composed_class_local(span, parts, &UncapturedSource).0,
                None => self.unsupported_class_names(value, span.start),
            },
            PreparedClassNames::Captured { captures, parts } => {
                let source: CapturedSource<'_, 'a> = &captures;
                let body = match parts {
                    Some(parts) => self.composed_class_local(span, parts, &source).0,
                    None => {
                        self.unsupported_class_names::<CapturedClassBody<'a>>(value, span.start)
                    }
                };
                FinishedClass::Call(crate::utils::call_with_values_box(
                    &self.ast,
                    captures.into_values(),
                    body.clone_expression(self.ast.allocator()),
                ))
            }
        }
    }

    fn unsupported_class_names<C: ClassConstructors<'a>>(
        &mut self,
        value: &Expression<'a>,
        offset: u32,
    ) -> C {
        self.errors.push((
            offset,
            element_error("ClassNames", &readable_code(value), CLASS_NAMES_PART),
        ));
        C::from_string(StringLiteral::boxed(SPAN, "", None, &self.ast))
    }

    fn composed_class_local<S: LocalSource<'a>>(
        &mut self,
        span: oxc_span::Span,
        parts: Vec<LocalKnownPart<'a, S::Class>>,
        source: &S,
    ) -> (S::Class, Option<Vec<ExtractStyleValue>>) {
        let offset = span.start;
        let mut composition: Composition<'a, S::Class> = Default::default();
        let mut classes = Vec::new();
        for part in parts {
            match part {
                LocalKnownPart::Styles(side) => {
                    let props = LocalRules::new(self, source).part_props_local(offset, side, None);
                    composition.apply(&self.ast, props);
                }
                LocalKnownPart::Conditional {
                    test,
                    consequent,
                    alternate,
                } => {
                    let consequent =
                        LocalRules::new(self, source).part_props_local(offset, consequent, None);
                    let alternate =
                        LocalRules::new(self, source).part_props_local(offset, alternate, None);
                    composition.apply_conditional(&self.ast, &test, consequent, alternate);
                }
                LocalKnownPart::Class(mut class) => {
                    class.read_in(&self.style_values, &self.ast);
                    classes.push(class);
                }
            }
        }
        let closed = classes.is_empty();
        let known = composition.unconditional().filter(|_| closed);
        let mut props = composition.into_normalised();
        props.reverse();
        let class_name = gen_normalised(
            &self.ast,
            &mut props,
            ClassPlacement {
                order: None,
                filename: self.split_filename.as_deref(),
            },
        );
        let result =
            merge_roots(&self.ast, classes.into_iter().chain(class_name)).unwrap_or_else(|| {
                S::Class::from_string(StringLiteral::boxed(SPAN, "", None, &self.ast))
            });
        if closed {
            let observation: Vec<ExtractStyleProp<'a>> = props
                .iter()
                .map(|prop| {
                    prop.clone_payload_in(self.ast.allocator(), |value, allocator| {
                        value.clone_expression(allocator)
                    })
                    .into_prop()
                })
                .collect();
            if let Some(finite) = crate::finite_styles::FiniteStyles::emitted(
                &self.ast,
                &observation,
                &result.clone_expression(self.ast.allocator()),
            ) {
                self.style_values.origin(span, finite);
            }
        }
        self.styles.extend(
            props
                .into_iter()
                .flat_map(|prop| prop.into_prop().into_extract_payload()),
        );
        (result, known)
    }
}

fn collect<'a, S: LocalSource<'a>>(
    visitor: &DevupVisitor<'a>,
    value: &Expression<'a>,
    source: &S,
) -> Option<Vec<LocalKnownPart<'a, S::Class>>> {
    let mut parts = Vec::new();
    LocalParts::new(visitor, source).known_parts_local(value, &mut parts, Text::Classes)?;
    Some(parts)
}

#[cfg(test)]
#[path = "class_names_emission/w38q_leaf_replay.rs"]
mod w38q_leaf_replay;
