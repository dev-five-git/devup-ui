//! Resolve Emotion namespace reads before the ordinary import alias pass.
mod acquire;
mod bindings;
mod declarations;
mod driver;
mod order;
mod output;
mod reads;
mod restore;
pub(super) mod units;
pub(super) use driver::normalize;
pub(super) use output::Rewritten;
pub(super) use restore::original_error_code;

use crate::{ImportAlias, import_alias_visit::Edit, utils::build_time_error};
use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::Expression};
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::{node::NodeId, symbol::SymbolId};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
};

/// Where a namespace came from. Only an ES module import can be lowered to
/// generated named imports; `require()` and `import x = require()` hand out a
/// runtime object, so their styling members are build errors.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Root {
    Module(Span),
    Require(Span),
}

#[derive(Clone, PartialEq, Eq)]
enum Binding {
    Namespace(Root),
    Macro(&'static str),
    String(String),
}

struct Normalizer<'s, 'a> {
    code: &'s str,
    semantic: &'s Semantic<'a>,
    bindings: HashMap<SymbolId, Binding>,
    aliases: HashSet<Span>,
    runtime: HashSet<Root>,
    loaders: HashSet<SymbolId>,
    module_api: HashSet<SymbolId>,
    names: HashSet<String>,
    imports: BTreeMap<&'static str, String>,
    replacements: Vec<(usize, usize, String)>,
    errors: Vec<(u32, String)>,
}

type Normalized<'a> = (Cow<'a, str>, Vec<Edit>);
type Errors = Vec<(u32, String)>;

const REQUIRE_REQUIREMENT: &str = "styling APIs read through require(), import-equals or dynamic import cannot be compiled; use `import * as ns from '@emotion/css'`";

impl Normalizer<'_, '_> {
    fn text(&self, span: Span) -> &str {
        &self.code[span.start as usize..span.end as usize]
    }

    fn error(&mut self, span: Span, requirement: &str) {
        self.errors.push((
            span.start,
            build_time_error("@emotion/css", self.text(span), requirement),
        ));
    }

    fn replace(&mut self, span: Span, value: String) {
        self.replacements
            .push((span.start as usize, span.end as usize, value));
    }

    fn is_runtime(&self, root: Root) -> bool {
        matches!(root, Root::Require(_)) || self.runtime.contains(&root)
    }

    fn symbol(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(identifier) = expression.get_inner_expression() else {
            return None;
        };
        identifier
            .reference_id
            .get()
            .and_then(|id| self.semantic.scoping().get_reference(id).symbol_id())
    }

    fn string(&self, expression: &Expression<'_>) -> Option<String> {
        match expression.get_inner_expression() {
            Expression::StringLiteral(string) => Some(string.value.to_string()),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
                .quasis
                .first()?
                .value
                .cooked
                .as_ref()
                .map(ToString::to_string),
            expression => match self.bindings.get(&self.symbol(expression)?)? {
                Binding::String(string) => Some(string.clone()),
                Binding::Namespace(_) | Binding::Macro(_) => None,
            },
        }
    }

    fn namespace_root(&self, expression: &Expression<'_>) -> Option<Root> {
        if let Some(root) = self.acquisition(expression) {
            return Some(root);
        }
        match self.bindings.get(&self.symbol(expression)?)? {
            Binding::Namespace(root) => Some(*root),
            Binding::Macro(_) | Binding::String(_) => None,
        }
    }

    fn member(&self, expression: &Expression<'_>) -> Option<(Root, Option<String>, bool)> {
        let (object, key, optional) = match expression.get_inner_expression() {
            Expression::StaticMemberExpression(member) => (
                &member.object,
                Some(member.property.name.to_string()),
                member.optional,
            ),
            Expression::ComputedMemberExpression(member) => (
                &member.object,
                self.string(&member.expression),
                member.optional,
            ),
            _ => return None,
        };
        Some((self.namespace_root(object)?, key, optional))
    }

    fn value(&self, expression: &Expression<'_>) -> Option<Binding> {
        if let Some(binding) = self
            .symbol(expression)
            .and_then(|symbol| self.bindings.get(&symbol))
        {
            return Some(binding.clone());
        }
        if let Some(root) = self.acquisition(expression) {
            return Some(Binding::Namespace(root));
        }
        if let Some((root, Some(key), false)) = self.member(expression) {
            return macro_name(&key)
                .filter(|_| matches!(root, Root::Module(_)))
                .map(Binding::Macro);
        }
        self.string(expression).map(Binding::String)
    }

    fn fresh(&mut self, api: &'static str) -> String {
        if let Some(name) = self.imports.get(api) {
            return name.clone();
        }
        let mut index = self.names.len();
        let name = loop {
            let name = format!("__emotion_{api}_{index}");
            if self.names.insert(name.clone()) {
                break name;
            }
            index += 1;
        };
        self.imports.insert(api, name.clone());
        name
    }

    /// The outermost syntax-only wrapper (`(x)`, `x as T`, `x!`, ...) of `node`.
    fn expression_node(&self, node: NodeId) -> NodeId {
        let mut current = node;
        for parent in self.semantic.nodes().ancestor_ids(node) {
            match self.semantic.nodes().kind(parent) {
                AstKind::ParenthesizedExpression(_)
                | AstKind::TSAsExpression(_)
                | AstKind::TSSatisfiesExpression(_)
                | AstKind::TSNonNullExpression(_)
                | AstKind::TSTypeAssertion(_)
                | AstKind::TSInstantiationExpression(_) => current = parent,
                _ => break,
            }
        }
        current
    }
}

fn macro_name(name: &str) -> Option<&'static str> {
    match name {
        "css" => Some("css"),
        "cx" => Some("cx"),
        "merge" => Some("merge"),
        "keyframes" => Some("keyframes"),
        "injectGlobal" => Some("injectGlobal"),
        _ => None,
    }
}

fn runtime_name(name: &str) -> bool {
    matches!(
        name,
        "cache" | "flush" | "hydrate" | "sheet" | "getRegisteredStyles"
    )
}

/// Every name the program binds or reads as a free variable
fn identifier_names(semantic: &Semantic<'_>) -> HashSet<String> {
    let scoping = semantic.scoping();
    scoping
        .symbol_names()
        .map(ToString::to_string)
        .chain(
            scoping
                .root_unresolved_references()
                .keys()
                .map(ToString::to_string),
        )
        .collect()
}
