use oxc_ast::{AstKind, ast::Program};
use oxc_semantic::Semantic;
use oxc_span::Span;

use super::{Demand, Index, MemberDemand, View, closure};
use crate::ordinary_ve::selection::plan::{Mutation, Read};

pub(super) fn seed(index: &Index<'_, '_>, reads: &[Span], view: &mut View) {
    for node in index.semantic.nodes().iter() {
        if let AstKind::IdentifierReference(identifier) = node.kind()
            && reads.contains(&identifier.span)
            && let Some(reference) = identifier.reference_id.get()
            && let Some(symbol) = index
                .semantic
                .scoping()
                .get_reference(reference)
                .symbol_id()
        {
            let mut demanded = Demand::default();
            match closure::demand(index, node.id()) {
                MemberDemand::Path(path) => {
                    demanded.insert(&path);
                }
                MemberDemand::Whole => {
                    demanded.insert(&[]);
                }
            }
            view.symbols.entry(symbol).or_default().merge(&demanded);
        }
    }
}

impl View {
    pub(crate) fn audit<'a>(
        &mut self,
        parsed: (&Program<'a>, &Semantic<'a>),
        option: &crate::ExtractOption,
    ) {
        let (program, semantic) = parsed;
        let mut styles =
            crate::imported_constants::consumer::style_names(program, semantic.scoping(), option);
        styles.extend(
            self.selection
                .imports
                .iter()
                .filter(|import| import.native.is_some())
                .map(|import| import.binding.name.clone()),
        );
        let compat = format!("{}/compat", option.package);
        let css =
            crate::css_prop::CssProp::of(&option.import_aliases, semantic.source_text(), false);
        self.selection.mutations =
            crate::mutations::uses(program, &|name| styles.contains(name), Some((css, &compat)))
                .into_iter()
                .filter_map(|(name, uses)| {
                    let symbol = semantic.scoping().get_root_binding(name.as_str().into())?;
                    self.symbols.contains_key(&symbol).then_some((symbol, uses))
                })
                .flat_map(|(symbol, uses)| {
                    uses.into_iter()
                        .map(move |usage| Mutation { symbol, usage })
                })
                .collect();
        self.selection
            .mutations
            .sort_by_key(|mutation| match &mutation.usage {
                crate::mutations::Use::Changes { at, .. }
                | crate::mutations::Use::Calls { at, .. }
                | crate::mutations::Use::Escapes { at, .. } => *at,
            });
        self.selection
            .reads
            .extend(semantic.nodes().iter().filter_map(|node| {
                let AstKind::IdentifierReference(identifier) = node.kind() else {
                    return None;
                };
                let data = semantic
                    .scoping()
                    .get_reference(identifier.reference_id.get()?);
                let symbol = data.symbol_id()?;
                (data.is_value() && self.symbols.contains_key(&symbol)).then(|| Read {
                    span: identifier.span,
                    symbol: Some(symbol),
                    name: identifier.name.to_string(),
                    write: data.is_write(),
                })
            }));
        self.selection.reads.sort_by_key(|read| read.span.start);
        self.selection.reads.dedup_by_key(|read| read.span);
    }
}
