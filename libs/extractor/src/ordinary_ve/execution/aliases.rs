use super::{Selection, Stylesheet};
use crate::mutations::Use;
use crate::utils::unwrap_syntax_only;
use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{BindingPattern, Expression, VariableDeclarationKind},
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) enum Alias {
    Readonly,
    Mutable(u32),
    Other,
}

pub(super) struct Aliases {
    targets: FxHashMap<String, String>,
    uses: FxHashMap<String, Vec<Use>>,
    selected: Vec<Span>,
}

impl Aliases {
    pub fn new(stylesheet: Stylesheet<'_>, selection: &Selection) -> Self {
        let allocator = Allocator::default();
        let parsed = Parser::new(
            &allocator,
            stylesheet.code,
            SourceType::from_path(stylesheet.filename).unwrap_or_default(),
        )
        .parse();
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(&parsed.program)
            .semantic;
        let scoping = semantic.scoping();
        let targets = semantic
            .nodes()
            .iter()
            .filter_map(|node| {
                let AstKind::VariableDeclarator(declarator) = node.kind() else {
                    return None;
                };
                let AstKind::VariableDeclaration(declaration) =
                    semantic.nodes().parent_kind(node.id())
                else {
                    return None;
                };
                if declaration.kind != VariableDeclarationKind::Const || declaration.declare {
                    return None;
                }
                let BindingPattern::BindingIdentifier(identifier) = &declarator.id else {
                    return None;
                };
                let symbol = identifier.symbol_id.get()?;
                if scoping.symbol_scope_id(symbol) != scoping.root_scope_id() {
                    return None;
                }
                let initializer = unwrap_syntax_only(declarator.init.as_ref()?);
                if let Some(member) = initializer.as_member_expression()
                    && member.static_property_name() == Some("exports")
                    && matches!(member.object(), Expression::Identifier(source)
                        if source.name == "module" && source.reference_id.get().is_some_and(|reference|
                            scoping.get_reference(reference).symbol_id().is_none()))
                {
                    return Some((identifier.name.to_string(), "module.exports".to_string()));
                }
                let Expression::Identifier(source) = initializer else { return None };
                if source.name == "exports"
                    && source.reference_id.get().is_some_and(|reference|
                        scoping.get_reference(reference).symbol_id().is_none())
                {
                    return Some((identifier.name.to_string(), "exports".to_string()));
                }
                let source_symbol = scoping
                    .get_reference(source.reference_id.get()?)
                    .symbol_id()?;
                if scoping.symbol_scope_id(source_symbol) != scoping.root_scope_id() {
                    return None;
                }
                Some((
                    identifier.name.to_string(),
                    scoping.symbol_name(source_symbol).to_string(),
                ))
            })
            .collect();
        let native: FxHashSet<_> = selection
            .imports
            .iter()
            .filter(|import| import.native.is_some())
            .map(|import| import.binding.name.as_str())
            .collect();
        Self {
            targets,
            uses: crate::mutations::uses(&parsed.program, &|name| native.contains(name), None),
            selected: selection.units.iter().map(|unit| unit.span).collect(),
        }
    }

    pub fn classify(&self, source: &str, alias: &str) -> Alias {
        if self
            .targets
            .get(alias)
            .is_none_or(|target| target != source)
        {
            return Alias::Other;
        }
        match self.mutable(alias, &mut FxHashSet::default()) {
            Some(at) => Alias::Mutable(at),
            None => Alias::Readonly,
        }
    }

    fn mutable(&self, alias: &str, seen: &mut FxHashSet<String>) -> Option<u32> {
        if !seen.insert(alias.to_string()) {
            return None;
        }
        for usage in self.uses.get(alias).into_iter().flatten() {
            let at = match usage {
                Use::Changes { at, .. } | Use::Calls { at, .. } | Use::Escapes { at, .. } => *at,
            };
            if self
                .selected
                .iter()
                .any(|span| (span.start..span.end).contains(&at))
            {
                continue;
            }
            match usage {
                Use::Escapes {
                    into: Some(into), ..
                } if into.is_empty() => {}
                Use::Escapes {
                    into: Some(into), ..
                } if self.targets.get(into).is_some_and(|target| target == alias) => {
                    if let Some(at) = self.mutable(into, seen) {
                        return Some(at);
                    }
                }
                Use::Changes { .. } | Use::Calls { .. } | Use::Escapes { .. } => return Some(at),
            }
        }
        None
    }
}
