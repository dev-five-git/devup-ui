use crate::{
    ExtractStyleProp,
    extract_style::extract_style_value::ExtractStyleValue,
    import_alias_visit::{Edit, source_offset},
};
use css::style_origin::{Origin, RealLocation, StyleOrigin};
use oxc_ast::ast::{Expression, Program};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_span::GetSpan;
use std::{cell::RefCell, collections::BTreeMap};

pub(crate) const MARKER: &str = "__devup_style_origin";

thread_local! {
    static ORIGINS: RefCell<BTreeMap<u32, Origin>> = RefCell::default();
    static CURRENT: RefCell<Origin> = RefCell::default();
}

pub(crate) struct CurrentOrigin(Origin);
impl CurrentOrigin {
    pub(crate) fn enter(span: oxc_span::Span) -> Self {
        let mut origin = for_span(span);
        origin.fill_from(&current());
        Self(CURRENT.with_borrow_mut(|current| std::mem::replace(current, origin)))
    }
}
impl Drop for CurrentOrigin {
    fn drop(&mut self) {
        CURRENT.with_borrow_mut(|current| *current = std::mem::take(&mut self.0));
    }
}
pub(crate) fn current() -> Origin {
    CURRENT.with_borrow(Clone::clone)
}

pub(crate) struct OriginScope(BTreeMap<u32, Origin>);

impl OriginScope {
    /// Capture before risky-span marking or constant replacement. Generated
    /// stylesheet spans are admitted only through explicit factory witnesses.
    pub(crate) fn enter(source: (&str, &str), edits: &[&[Edit]], program: &Program<'_>) -> Self {
        let mut captured = BTreeMap::new();
        Capture {
            file: source.0,
            source: source.1,
            edits,
            captured: &mut captured,
        }
        .visit_program(program);
        Self::install(captured)
    }

    pub(crate) fn generated(program: &mut Program<'_>) -> Self {
        let mut captured = BTreeMap::new();
        Markers(&mut captured).visit_program(program);
        Self::install(captured)
    }

    fn install(captured: BTreeMap<u32, Origin>) -> Self {
        Self(ORIGINS.with_borrow_mut(|origins| std::mem::replace(origins, captured)))
    }
}

impl Drop for OriginScope {
    fn drop(&mut self) {
        ORIGINS.with_borrow_mut(|origins| *origins = std::mem::take(&mut self.0));
    }
}

pub(crate) fn at(start: u32) -> Origin {
    ORIGINS
        .with_borrow(|origins| {
            origins
                .get(&crate::provenance::source_offset(start))
                .cloned()
        })
        .unwrap_or_default()
}

pub(crate) fn for_span(span: oxc_span::Span) -> Origin {
    if span == oxc_span::SPAN {
        Origin::default()
    } else {
        at(span.start)
    }
}

pub(crate) fn actual(file: &str, source: &str, span: oxc_span::Span) -> Option<StyleOrigin> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    let before = source.get(..start)?;
    let expression = source.get(start..end)?;
    Some(StyleOrigin {
        file: file.to_string(),
        line: before.bytes().filter(|byte| *byte == b'\n').count() + 1,
        column: before
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            + 1,
        expression: expression.to_string(),
    })
}

struct Capture<'a> {
    file: &'a str,
    source: &'a str,
    edits: &'a [&'a [Edit]],
    captured: &'a mut BTreeMap<u32, Origin>,
}

impl<'a> Visit<'a> for Capture<'_> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        self.capture(expression.span());
        walk::walk_expression(self, expression);
    }
    fn visit_string_literal(&mut self, literal: &oxc_ast::ast::StringLiteral<'a>) {
        self.capture(literal.span);
    }
    fn visit_template_literal(&mut self, template: &oxc_ast::ast::TemplateLiteral<'a>) {
        self.capture(template.span);
        walk::walk_template_literal(self, template);
    }
}
impl Capture<'_> {
    fn capture(&mut self, span: oxc_span::Span) {
        if let (Ok(start), Ok(end)) = (usize::try_from(span.start), usize::try_from(span.end))
            && let (Ok(start), Ok(end)) = (
                u32::try_from(
                    self.edits
                        .iter()
                        .fold(start, |offset, edits| source_offset(edits, offset)),
                ),
                u32::try_from(
                    self.edits
                        .iter()
                        .fold(end, |offset, edits| source_end(edits, offset)),
                ),
            )
            && let Some(origin) = actual(self.file, self.source, oxc_span::Span::new(start, end))
        {
            self.captured
                .entry(span.start)
                .or_insert_with(|| Origin(Some(Box::new(origin)), None));
        }
    }
}

fn source_end(edits: &[Edit], offset: usize) -> usize {
    let (mut added, mut removed) = (0, 0);
    for &(start, end, length) in edits {
        let replaced_at = start + added - removed;
        if offset <= replaced_at {
            break;
        }
        if offset <= replaced_at + length {
            return end;
        }
        added += length;
        removed += end - start;
    }
    offset + removed - added
}

struct FillRange<'a> {
    origin: &'a Origin,
    captured: &'a mut BTreeMap<u32, Origin>,
}
impl<'a> Visit<'a> for FillRange<'_> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        self.captured
            .insert(expression.span().start, self.origin.clone());
        walk::walk_expression(self, expression);
    }
}

struct Markers<'a>(&'a mut BTreeMap<u32, Origin>);
impl<'a> VisitMut<'a> for Markers<'_> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if let Expression::CallExpression(call) = expression
            && matches!(&call.callee, Expression::Identifier(name) if name.name == MARKER)
            && let [oxc_ast::ast::Argument::StringLiteral(origin), value] =
                call.arguments.as_slice()
            && let Ok(location) =
                serde_json::from_str::<RealLocation>(&origin.value).or_else(|_| {
                    serde_json::from_str::<StyleOrigin>(&origin.value).map(RealLocation::Exact)
                })
            && let Some(value) = value.as_expression()
        {
            FillRange {
                origin: &Origin::from_location(location),
                captured: self.0,
            }
            .visit_expression(value);
            let value = call.arguments.pop();
            if let Some(value) = value.and_then(|value| Expression::try_from(value).ok()) {
                *expression = value;
            }
        }
        walk_mut::walk_expression(self, expression);
    }
}

#[derive(Clone)]
struct EvaluationSource {
    file: String,
    source: String,
    edits: Vec<Edit>,
}
thread_local! {
    static EVALUATION_SOURCE: RefCell<Option<EvaluationSource>> = const { RefCell::new(None) };
}
pub(crate) struct EvaluationScope(Option<EvaluationSource>);
impl EvaluationScope {
    pub(crate) fn enter(file: &str, source: &str, edits: &[Edit]) -> Self {
        Self(EVALUATION_SOURCE.with_borrow_mut(|current| {
            current.replace(EvaluationSource {
                file: file.to_string(),
                source: source.to_string(),
                edits: edits.to_vec(),
            })
        }))
    }
}
impl Drop for EvaluationScope {
    fn drop(&mut self) {
        EVALUATION_SOURCE.with_borrow_mut(|current| *current = self.0.take());
    }
}
pub(crate) fn evaluated_at(file: &str, source: &str, span: oxc_span::Span) -> Option<StyleOrigin> {
    EVALUATION_SOURCE.with_borrow(|current| {
        match current.as_ref().filter(|current| current.file == file) {
            None => actual(file, source, span),
            Some(current) => {
                let start = u32::try_from(source_offset(
                    &current.edits,
                    usize::try_from(span.start).ok()?,
                ))
                .ok()?;
                let end =
                    u32::try_from(source_end(&current.edits, usize::try_from(span.end).ok()?))
                        .ok()?;
                actual(file, &current.source, oxc_span::Span::new(start, end))
            }
        }
    })
}

pub(crate) fn fill_style(style: &mut ExtractStyleValue, origin: &Origin) {
    match style {
        ExtractStyleValue::Static(style) => style.origin.fill_from(origin),
        ExtractStyleValue::Dynamic(style) => style.origin.fill_from(origin),
        ExtractStyleValue::Keyframes(style) => style.origin.fill_from(origin),
        ExtractStyleValue::Typography(_)
        | ExtractStyleValue::Css(_)
        | ExtractStyleValue::Import(_)
        | ExtractStyleValue::FontFace(_) => {}
    }
}

pub(crate) fn fill(props: &mut [ExtractStyleProp<'_>], origin: &Origin) {
    for prop in props {
        match prop {
            ExtractStyleProp::Static(style) => fill_style(style, origin),
            ExtractStyleProp::StaticArray(props)
            | ExtractStyleProp::Evaluated { styles: props, .. } => fill(props, origin),
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for branch in [consequent, alternate].into_iter().flatten() {
                    fill(std::slice::from_mut(branch.as_mut()), origin);
                }
            }
            ExtractStyleProp::Enum { map, .. } => {
                for props in map.values_mut() {
                    fill(props, origin);
                }
            }
            ExtractStyleProp::MemberExpression { map, .. } => {
                for prop in map.values_mut() {
                    fill(std::slice::from_mut(prop.as_mut()), origin);
                }
            }
            ExtractStyleProp::Expression { styles, .. } => {
                for style in styles {
                    fill_style(style, origin);
                }
            }
            ExtractStyleProp::Unreadable { .. } => {}
        }
    }
}
