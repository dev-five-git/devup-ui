use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BTreeMap, BTreeSet};

use super::{
    CollectedStyles, Stylesheet,
    naming::{ExportValue, Representation},
};
use analysis::Index;
use safety::Disposition;

mod alias_candidates;
mod aliases;
mod analysis;
mod compiled;
mod effects;
mod imports;
mod regions;
mod retention;
mod safety;
pub(super) use compiled::Origin;

#[derive(Default)]
pub(crate) struct Output {
    pub code: String,
    pub reserved: BTreeSet<String>,
}

struct Piece {
    span: Span,
    text: String,
    export: String,
}

pub(super) fn prepare(
    input: (Stylesheet<'_>, Origin<'_>),
    collected: &mut CollectedStyles,
    exports: &[ExportValue],
) -> Result<Output, String> {
    let (stylesheet, origin) = input;
    let fallbacks: Vec<_> = exports
        .iter()
        .filter(|export| match export.representation {
            Representation::Authored => true,
            Representation::Generated | Representation::Serialized => false,
        })
        .collect();
    if fallbacks.is_empty() {
        return Ok(Output::default());
    }
    let allocator = Allocator::default();
    let program = Parser::new(
        &allocator,
        stylesheet.code,
        SourceType::from_path(stylesheet.filename).unwrap_or_default(),
    )
    .parse()
    .program;
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;
    let index = Index::new(&program, &semantic, stylesheet.code);
    let scoping = semantic.scoping();
    let reserved = super::naming::reserved_names(scoping);
    let mut dispositions: FxHashMap<_, _> = scoping
        .symbol_ids()
        .filter(|symbol| scoping.symbol_scope_id(*symbol) == scoping.root_scope_id())
        .map(|symbol| {
            let name = scoping.symbol_name(symbol);
            let rewritten = collected.styles.contains_key(name)
                || collected.keyframes.contains_key(name)
                || collected
                    .constant_exports
                    .iter()
                    .any(|(binding, _)| binding == name);
            (
                symbol,
                if rewritten {
                    Disposition::Rewritten
                } else {
                    Disposition::Removed
                },
            )
        })
        .collect();
    let mut pieces = BTreeMap::new();
    let mut retained = FxHashSet::default();
    for fallback in &fallbacks {
        if let Some(export) = index.exports.get(&fallback.name) {
            if let Some(symbol) = export.target {
                retention::binding(
                    (symbol, &fallback.name, &index),
                    (&mut dispositions, &mut retained),
                    &mut pieces,
                );
            }
            if !export.text.is_empty() {
                pieces
                    .entry((export.span.start, export.span.end))
                    .or_insert_with(|| Piece {
                        span: export.span,
                        text: export.text.clone(),
                        export: fallback.name.clone(),
                    });
            }
        } else if !index.stars.is_empty() {
            for span in &index.stars {
                pieces
                    .entry((span.start, span.end))
                    .or_insert_with(|| Piece {
                        span: *span,
                        text: span.source_text(stylesheet.code).to_string(),
                        export: fallback.name.clone(),
                    });
            }
        } else {
            return Err(crate::located_errors(
                stylesheet.filename,
                stylesheet.source,
                stylesheet.edits,
                vec![(
                    0,
                    format!(
                        "stylesheet export `{}` has no authored fallback site. Fix: retain its authored declaration in the stylesheet",
                        fallback.name
                    ),
                )],
            ));
        }
    }
    let related = aliases::related(&index, &dispositions);
    let accounting = compiled::Accounting::new((&program, &semantic), &origin, &related);
    aliases::retain(
        (stylesheet, &program, &index),
        (&mut dispositions, &mut retained),
        (&mut pieces, &accounting),
    )?;
    effects::retain(
        (&program, &index, &accounting),
        (&mut dispositions, &mut retained, &related),
        &mut pieces,
    );
    imports::retain(
        (&program, &index, &origin),
        &mut dispositions,
        (&mut pieces, &accounting),
    );
    let restored = retention::exports(&index, &dispositions, &mut pieces);
    let pieces: Vec<_> = pieces.into_values().collect();
    safety::check((stylesheet, &index), &pieces, &dispositions)?;
    collected
        .constant_exports
        .retain(|(name, _)| !retained.contains(name));
    collected.export_aliases.retain(|(_, alias, _)| {
        !restored.contains(alias) && !fallbacks.iter().any(|fallback| fallback.name == *alias)
    });
    Ok(Output {
        code: pieces
            .into_iter()
            .map(|piece| piece.text)
            .collect::<Vec<_>>()
            .join("\n"),
        reserved,
    })
}
