use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::{Piece, analysis::Index};
use crate::vanilla_extract::Stylesheet;

pub(super) enum Disposition {
    Retained,
    CompilerInput,
    Rewritten,
    Removed,
}

pub(super) fn check(
    input: (Stylesheet<'_>, &Index<'_, '_>),
    pieces: &[Piece],
    dispositions: &FxHashMap<SymbolId, Disposition>,
) -> Result<(), String> {
    let (stylesheet, index) = input;
    let mut errors = Vec::new();
    for (symbol, disposition) in dispositions {
        let cause = match disposition {
            Disposition::Retained => continue,
            Disposition::Rewritten => "rewritten",
            Disposition::CompilerInput | Disposition::Removed => "removed",
        };
        for id in index.semantic.scoping().get_resolved_reference_ids(*symbol) {
            let reference = index.semantic.scoping().get_reference(*id);
            if !reference.is_value() {
                continue;
            }
            let span = index.semantic.nodes().kind(reference.node_id()).span();
            if matches!(disposition, Disposition::CompilerInput)
                && index
                    .semantic
                    .nodes()
                    .ancestor_kinds(reference.node_id())
                    .any(|kind| match kind {
                        oxc_ast::AstKind::CallExpression(call) => {
                            call.callee.span().contains_inclusive(span)
                        }
                        oxc_ast::AstKind::TaggedTemplateExpression(tag) => {
                            tag.tag.span().contains_inclusive(span)
                        }
                        oxc_ast::AstKind::JSXOpeningElement(element) => {
                            element.name.span().contains_inclusive(span)
                        }
                        oxc_ast::AstKind::JSXClosingElement(element) => {
                            element.name.span().contains_inclusive(span)
                        }
                        _ => false,
                    })
            {
                continue;
            }
            if let Some(piece) = pieces
                .iter()
                .find(|piece| piece.span.contains_inclusive(span))
            {
                let name = index.semantic.scoping().symbol_name(*symbol);
                errors.push((span.start, format!(
                    "stylesheet export `{}` cannot use `{name}` at build time: authored fallback references binding `{name}` {cause} by stylesheet compilation. Fix: make the export self-contained or move its runtime dependency to an ordinary module",
                    piece.export
                )));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(crate::located_errors(
            stylesheet.filename,
            stylesheet.source,
            stylesheet.edits,
            errors,
        ))
    }
}
