use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{BindingPattern, Expression, ObjectPropertyKind},
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};

use super::{Demand, View, render::Rendered};
use crate::vanilla_extract::Stylesheet;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct Observed {
    pub view: View,
    pub rendered: Rendered,
    pub type_reads: Vec<(String, Span)>,
}

pub(super) fn observe(filename: &str, source: &str, demand: &Demand) -> TestResult<Observed> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    let view = super::select_reads(
        (&parsed.program, &built.semantic),
        (demand, &[]),
        (filename, "@vanilla-extract/css", None),
    );
    let mut type_reads = Vec::new();
    for node in built.semantic.nodes().iter() {
        if let AstKind::IdentifierReference(identifier) = node.kind() {
            let reference = identifier
                .reference_id
                .get()
                .ok_or("missing real reference")?;
            if !built.semantic.scoping().get_reference(reference).is_value() {
                type_reads.push((identifier.name.to_string(), identifier.span));
            }
        }
    }
    let rendered = view.render(
        Stylesheet {
            filename,
            code: source,
            source,
            edits: &[],
        },
        &crate::ExtractOption::default(),
    )?;
    Ok(Observed {
        view,
        rendered,
        type_reads,
    })
}

pub(super) fn path(keys: &[&str]) -> Demand {
    keys.iter()
        .rev()
        .fold(Demand::whole(), |child, key| Demand::prefixed(key, &child))
}

pub(super) fn span(source: &str, text: &str) -> TestResult<Span> {
    let start = source.find(text).ok_or("fixture span missing")?;
    let end = start
        .checked_add(text.len())
        .ok_or("fixture span overflow")?;
    Ok(Span::new(u32::try_from(start)?, u32::try_from(end)?))
}

pub(super) fn assert_identity(
    observed: &Observed,
    source: &str,
    declarations: &[(&str, &str)],
) -> TestResult {
    assert_eq!(observed.view.selection.units.len(), declarations.len());
    assert_eq!(observed.rendered.bindings.len(), declarations.len());
    for ((unit, rendered), (name, declaration)) in observed
        .view
        .selection
        .units
        .iter()
        .zip(&observed.rendered.bindings)
        .zip(declarations)
    {
        assert_eq!(unit.span, span(source, declaration)?);
        let [binding] = unit.bindings.as_slice() else {
            panic!("expected one real binding")
        };
        assert_eq!(binding.name, *name);
        assert_eq!(binding.span, span(source, name)?);
        assert_eq!(rendered.name, *name);
        assert_eq!(rendered.span, span(source, name)?);
    }
    assert_eq!(
        observed.view.exports,
        vec![("tokens".into(), "tokens".into())]
    );
    assert_eq!(observed.rendered.native.len(), 0);
    Ok(())
}

pub(super) fn objects(rendered: &Rendered) -> TestResult<Vec<(String, Vec<Option<String>>)>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &rendered.mapped.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", rendered.mapped.code);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{}", rendered.mapped.code);
    let mut objects = Vec::new();
    for node in built.semantic.nodes().iter() {
        let AstKind::VariableDeclarator(declarator) = node.kind() else {
            continue;
        };
        let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
            continue;
        };
        let Some(init) = &declarator.init else {
            continue;
        };
        let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(init) else {
            continue;
        };
        let properties = object
            .properties
            .iter()
            .map(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => Some(
                    property
                        .key
                        .static_name()
                        .ok_or("rendered property has no static name")
                        .map(std::borrow::Cow::into_owned),
                ),
                ObjectPropertyKind::SpreadProperty(_) => None,
            })
            .map(Option::transpose)
            .collect::<Result<Vec<_>, _>>()?;
        objects.push((binding.name.to_string(), properties));
    }
    Ok(objects)
}
