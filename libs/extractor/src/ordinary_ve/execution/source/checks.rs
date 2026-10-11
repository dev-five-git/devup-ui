use oxc_ast::{AstKind, ast::VariableDeclarationKind};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};

use super::super::{SelectedModule, imports, policy};
use crate::ordinary_ve::selection::plan::UnitKind;
use crate::vanilla_extract::json_string;

pub(super) fn build(module: SelectedModule<'_>, semantic: &Semantic<'_>) -> Vec<(Span, String)> {
    let SelectedModule {
        stylesheet,
        selection,
    } = module;
    let expression = |span: Span, code: String| {
        let shorthand = semantic.nodes().iter().any(|node| matches!(node.kind(),
            AstKind::ObjectProperty(property) if property.shorthand && property.value.span() == span));
        let prefix = if shorthand {
            format!("{}: ", span.source_text(stylesheet.code))
        } else {
            String::new()
        };
        (span, format!("{prefix}{code}"))
    };
    let mut checks: Vec<_> = selection
        .checks
        .iter()
        .map(|escape| {
            let message = format!(
                "{}: {}. Fix: {}",
                policy::place(stylesheet, escape.span.start),
                escape.cause(),
                escape.fix()
            );
            expression(
                escape.span,
                format!("(function(){{throw {};}})()", json_string(&message)),
            )
        })
        .collect();
    // Boa can attribute a bound TDZ read to its enclosing property. An immediate
    // thunk gives that read its own frame, anchored through the existing trace.
    for read in selection.reads.iter().filter(|read| !read.write) {
        let forward = read.symbol.is_some_and(|symbol| {
            selection.units.iter().any(|unit| {
                matches!(
                    unit.kind,
                    UnitKind::Declarator {
                        kind: VariableDeclarationKind::Const | VariableDeclarationKind::Let,
                        ..
                    }
                ) && unit
                    .bindings
                    .iter()
                    .any(|binding| binding.symbol == symbol && binding.span.start > read.span.start)
            })
        });
        if forward && read.name != "eval" && !checks.iter().any(|(span, _)| *span == read.span) {
            checks.push(expression(read.span, format!("(()=>{})()", read.name)));
        }
    }
    for call in &selection.native_calls {
        if !checks
            .iter()
            .any(|(span, _)| call.callee.contains_inclusive(*span))
        {
            checks.push((
                call.callee,
                imports::native_read(stylesheet.code, selection, call),
            ));
        }
    }
    checks.sort_by_key(|(span, _)| span.start);
    checks
}
