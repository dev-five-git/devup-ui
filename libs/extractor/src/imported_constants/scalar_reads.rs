//! Eager own-slot projections, never aggregate or closure safety exemptions.

use super::{Constant, ModuleScope, provenance::Proof, scalar_literals::own_scalar};
use crate::{
    css_prop::binding_of,
    mutations::{Use, callees},
    utils::unwrap_syntax_only,
};
use oxc_ast::{
    AstKind,
    ast::{Expression, ObjectPropertyKind, Statement, VariableDeclarationKind, VariableDeclarator},
};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, Span};
use oxc_syntax::{reference::ReferenceId, symbol::SymbolId};
use rustc_hash::FxHashMap;

pub(super) struct Read {
    span: Span,
    symbol: SymbolId,
    reference: ReferenceId,
    key: Option<String>,
}

impl Read {
    pub(super) fn new(expression: &Expression<'_>, scoping: &Scoping) -> Option<Self> {
        let (identifier, key) = match expression {
            Expression::CallExpression(call) if !call.optional && call.arguments.is_empty() => {
                let Expression::Identifier(identifier) = &call.callee else {
                    return None;
                };
                (&**identifier, None)
            }
            _ => {
                let (identifier, key) = member(expression)?;
                (identifier, Some(key))
            }
        };
        Some(Self {
            span: expression.span(),
            symbol: binding_of(scoping, identifier)?,
            reference: identifier.reference_id.get()?,
            key,
        })
    }
}

fn member<'s, 'a>(
    expression: &'s Expression<'a>,
) -> Option<(&'s oxc_ast::ast::IdentifierReference<'a>, String)> {
    let (object, key) = match unwrap_syntax_only(expression) {
        Expression::StaticMemberExpression(member) if !member.optional => {
            (&member.object, member.property.name.to_string())
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            let Expression::StringLiteral(key) = unwrap_syntax_only(&member.expression) else {
                return None;
            };
            (&member.object, key.value.to_string())
        }
        _ => return None,
    };
    let Expression::Identifier(identifier) = unwrap_syntax_only(object) else {
        return None;
    };
    Some((identifier, key))
}

pub(super) fn resolve(scope: &ModuleScope<'_, '_>, reads: &[Read]) -> FxHashMap<Span, Constant> {
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(scope.program)
        .semantic;
    let proof = Proof {
        nodes: semantic.nodes(),
        scoping: semantic.scoping(),
    };
    let uses = scope.uses.as_deref();
    let projection = Projection { proof, scope, uses };
    reads
        .iter()
        .filter_map(|read| Some((read.span, projection.read(read)?)))
        .collect()
}

struct Projection<'s, 'p, 'a> {
    proof: Proof<'s, 'a>,
    scope: &'s ModuleScope<'p, 'a>,
    uses: Option<&'s FxHashMap<String, Vec<Use>>>,
}

impl<'a> Projection<'_, '_, 'a> {
    fn declaration(&self, symbol: SymbolId, at: u32) -> Option<&VariableDeclarator<'a>> {
        if self.proof.scoping.symbol_scope_id(symbol) != self.proof.scoping.root_scope_id()
            || self.scope.program.body.iter().any(|statement| matches!(statement,
                Statement::ExportNamedDeclaration(export) if export.specifiers.iter().any(|specifier|
                    self.proof.scoping.get_root_binding(specifier.local.name().as_str().into()) == Some(symbol))))
            || self.proof.scoping.get_resolved_reference_ids(symbol).iter().any(|id| self.proof.scoping.get_reference(*id).is_write()) {
            return None;
        }
        self.scope.program.body.iter().find_map(|statement| {
            let Statement::VariableDeclaration(declaration) = statement else {
                return None;
            };
            if declaration.kind != VariableDeclarationKind::Const {
                return None;
            }
            declaration.declarations.iter().find(|declaration| {
                declaration.span.end <= at
                    && declaration
                        .id
                        .get_binding_identifier()
                        .and_then(|id| id.symbol_id.get())
                        == Some(symbol)
            })
        })
    }

    fn eager(&self, reference: ReferenceId) -> bool {
        let mut node = self.proof.scoping.get_reference(reference).node_id();
        loop {
            match self.proof.nodes.kind(node) {
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) | AstKind::Class(_) => {
                    return false;
                }
                AstKind::Program(_) => return true,
                _ => node = self.proof.nodes.parent_id(node),
            }
        }
    }

    fn eager_hazard(&self, symbol: SymbolId, at: u32) -> bool {
        self.proof
            .scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .any(|id| {
                self.proof
                    .nodes
                    .kind(self.proof.scoping.get_reference(*id).node_id())
                    .span()
                    .start
                    == at
                    && self.eager(*id)
            })
    }

    fn found(&self, symbol: SymbolId) -> &[Use] {
        self.uses
            .and_then(|uses| uses.get(self.proof.scoping.symbol_name(symbol)))
            .map_or(&[], Vec::as_slice)
    }

    fn read(&self, read: &Read) -> Option<Constant> {
        if !self.eager(read.reference) {
            return None;
        }
        let (root, key) = if let Some(key) = &read.key {
            (read.symbol, key.clone())
        } else {
            self.declaration(read.symbol, read.span.start)?;
            if self
                .found(read.symbol)
                .iter()
                .any(|found| !matches!(found, Use::Calls { .. }))
            {
                return None;
            }
            let (identifier, key) = member(self.proof.factory(read.symbol)?)?;
            (binding_of(self.proof.scoping, identifier)?, key)
        };
        let declaration = self.declaration(root, read.span.start)?;
        let init = unwrap_syntax_only(declaration.init.as_ref()?);
        let (object, frozen) = match init {
            Expression::ObjectExpression(object) => (object, false),
            Expression::CallExpression(call)
                if !call.optional
                    && call.arguments.len() == 1
                    && callees::global(&self.proof, &call.callee) == Some(("Object", "freeze")) =>
            {
                let Expression::ObjectExpression(object) =
                    unwrap_syntax_only(call.arguments.first()?.as_expression()?)
                else {
                    return None;
                };
                (object, true)
            }
            _ => return None,
        };
        let value =
            if let [ObjectPropertyKind::SpreadProperty(spread)] = object.properties.as_slice() {
                let Expression::Identifier(source) = unwrap_syntax_only(&spread.argument) else {
                    return None;
                };
                let source = binding_of(self.proof.scoping, source)?;
                let source_declaration = self.declaration(source, declaration.span.start)?;
                let Expression::ObjectExpression(original) =
                    unwrap_syntax_only(source_declaration.init.as_ref()?)
                else {
                    return None;
                };
                if !self.proof.binding(source).shallow_primitives()
                    || !self.copy_safe(source, declaration.span.end)
                {
                    return None;
                }
                own_scalar(original, &key)?
            } else {
                if !self.proof.expression(init).plain() {
                    return None;
                }
                own_scalar(object, &key)?
            };
        self.slot_safe(root, &key, frozen).then_some(value)
    }

    fn copy_safe(&self, source: SymbolId, at: u32) -> bool {
        let shape = self.proof.binding(source);
        self.found(source).iter().all(|found| match found {
            Use::Escapes { path, .. } | Use::Calls { path, .. } if shape.primitive_path(path) => {
                true
            }
            Use::Escapes {
                at: hazard,
                into: None,
                ..
            }
            | Use::Changes { at: hazard, .. }
            | Use::Calls { at: hazard, .. } => *hazard > at && self.eager_hazard(source, *hazard),
            Use::Escapes { into: Some(_), .. } => false,
        })
    }

    fn slot_safe(&self, root: SymbolId, key: &str, frozen: bool) -> bool {
        let shape = self.proof.binding(root);
        self.found(root).iter().all(|found| match found {
            Use::Escapes { path, .. } | Use::Calls { path, .. } if shape.primitive_path(path) => {
                true
            }
            Use::Changes { .. } | Use::Escapes { into: Some(_), .. } => false,
            Use::Escapes {
                at,
                path,
                into: None,
            }
            | Use::Calls { at, path } => {
                if path.iter().any(Option::is_none) || !self.eager_hazard(root, *at) {
                    return false;
                }
                if path
                    .first()
                    .is_some_and(|first| first.as_deref() != Some(key))
                {
                    return true;
                }
                frozen
                    && self.uses.is_some_and(|uses| {
                        super::freeze::before(&self.proof, self.scope.program, root, *at, uses)
                    })
            }
        })
    }
}
