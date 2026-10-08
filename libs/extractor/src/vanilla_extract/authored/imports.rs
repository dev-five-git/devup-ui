use oxc_ast::ast::{ImportDeclarationSpecifier, Program, Statement};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

use super::{
    Piece,
    analysis::Index,
    compiled::{Accounting, Origin},
    safety::Disposition,
};

pub(super) fn retain(
    input: (&Program<'_>, &Index<'_, '_>, &Origin<'_>),
    dispositions: &mut FxHashMap<SymbolId, Disposition>,
    output: (&mut BTreeMap<(u32, u32), Piece>, &Accounting),
) {
    let (program, index, origin) = input;
    let (pieces, accounting) = output;
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        if import.import_kind.is_type() {
            continue;
        }
        let mut named = Vec::new();
        let mut standalone = Vec::new();
        for specifier in import.specifiers.iter().flatten() {
            let Some(symbol) = specifier.local().symbol_id.get() else {
                continue;
            };
            if !index
                .semantic
                .scoping()
                .get_resolved_reference_ids(symbol)
                .iter()
                .any(|id| {
                    let reference = index.semantic.scoping().get_reference(*id);
                    reference.is_value()
                        && pieces.values().any(|piece| {
                            piece.span.contains_inclusive(
                                index.semantic.nodes().kind(reference.node_id()).span(),
                            )
                        })
                })
            {
                continue;
            }
            let compiler = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    if specifier.import_kind.is_type() {
                        continue;
                    }
                    import.source.value == origin.package
                        && (crate::util_type::UtilType::from_str_opt(
                            specifier.imported.name().as_str(),
                        )
                        .is_some()
                            || specifier.imported.name() == "styled"
                            || specifier
                                .imported
                                .name()
                                .parse::<crate::component::ExportVariableKind>()
                                .is_ok())
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => false,
            };
            if accounting.is_native(symbol) && !compiler {
                continue;
            }
            if crate::package_specifier::is_package(import.source.value.as_str(), origin.package)
                && !compiler
            {
                continue;
            }
            dispositions.insert(
                symbol,
                if compiler {
                    Disposition::CompilerInput
                } else {
                    Disposition::Retained
                },
            );
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(_) => {
                    named.push(specifier.span().source_text(index.code));
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                    standalone.push(specifier.span().source_text(index.code));
                }
            }
        }
        if named.is_empty() && standalone.is_empty() {
            continue;
        }
        let source = Span::new(import.source.span.start, import.span.end)
            .source_text(index.code)
            .trim_end_matches(';');
        let mut declarations: Vec<_> = standalone
            .iter()
            .map(|specifier| format!("import {specifier} from {source};"))
            .collect();
        if !named.is_empty() {
            declarations.push(format!("import {{ {} }} from {source};", named.join(", ")));
        }
        pieces
            .entry((import.span.start, import.span.end))
            .or_insert_with(|| Piece {
                span: import.span,
                text: declarations.join("\n"),
                export: "import".into(),
            });
    }
}
