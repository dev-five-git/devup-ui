//! Original-source identities passed from selection to mapped execution.

use std::collections::BTreeSet;

use oxc_ast::ast::VariableDeclarationKind;
use oxc_span::Span;
use oxc_syntax::{node::NodeId, symbol::SymbolId};

/// A copyable source unit. Declarators never include their siblings.
#[derive(Debug, Clone)]
pub(crate) struct Unit {
    pub node: NodeId,
    pub span: Span,
    pub kind: UnitKind,
    pub bindings: Vec<Binding>,
}

#[derive(Debug, Clone)]
pub(crate) enum UnitKind {
    Declarator {
        kind: VariableDeclarationKind,
        pattern: Span,
        erased: bool,
    },
    /// Copy at its original position; JS, not the selector, supplies hoisting.
    Function {
        erased: bool,
    },
    Statement,
}

#[derive(Debug, Clone)]
pub(crate) struct Binding {
    pub symbol: SymbolId,
    pub span: Span,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeBinding {
    Named { api: &'static str },
    Namespace,
}

#[derive(Debug, Clone)]
pub(crate) enum ImportName {
    Named(String),
    Default,
    Namespace,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportBinding {
    pub binding: Binding,
    pub declaration: Span,
    pub specifier: Span,
    pub source: String,
    pub imported: ImportName,
    pub erased: bool,
    pub native: Option<NativeBinding>,
}

/// A lexical read, not an assertion that its branch executes.
#[derive(Debug, Clone)]
pub(crate) struct Read {
    pub span: Span,
    pub symbol: Option<SymbolId>,
    pub name: String,
    pub write: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum MemberDemand {
    Path(Vec<String>),
    Whole,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportDemand {
    pub symbol: SymbolId,
    pub read: Span,
    pub member: MemberDemand,
}

#[derive(Debug)]
pub(crate) struct Mutation {
    pub symbol: SymbolId,
    pub usage: crate::mutations::Use,
}

#[derive(Debug, Clone)]
pub(crate) struct NativeCall {
    pub node: NodeId,
    pub span: Span,
    pub callee: Span,
    pub api: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct HelperCall {
    pub caller: NodeId,
    pub callable: NodeId,
    pub span: Span,
}

/// The outer initializer/statement owns all nested native calls and captures.
#[derive(Debug, Clone)]
pub(crate) struct Root {
    pub owner: NodeId,
    pub span: Span,
    /// Sites are unique syntax sites, NOT an allocation or invocation schedule.
    pub native_calls: Vec<NodeId>,
    /// Read these already-computed bindings; never replay an initializer/default.
    pub captures: Vec<Binding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EscapeKind {
    NativeValue,
    DynamicNamespace,
    RuntimeHelper,
}

/// Structural escapes have original causes; executed unknown reads belong to
/// the existing sandbox and use `Read`, not speculative selection errors.
#[derive(Debug, Clone)]
pub(crate) struct Escape {
    pub span: Span,
    pub symbol: SymbolId,
    pub kind: EscapeKind,
}

impl Escape {
    pub(crate) const fn cause(&self) -> &'static str {
        match self.kind {
            EscapeKind::NativeValue => {
                "a native styling API escapes its exact initialization slice"
            }
            EscapeKind::DynamicNamespace => "a native styling namespace needs a static API member",
            EscapeKind::RuntimeHelper => "a native styling helper remains reachable at runtime",
        }
    }

    pub(crate) const fn fix(&self) -> &'static str {
        match self.kind {
            EscapeKind::NativeValue => {
                "call the API in an exact initializer instead of exporting or handing off the API"
            }
            EscapeKind::DynamicNamespace => "select the API with a static name or literal key",
            EscapeKind::RuntimeHelper => {
                "call the helper only from exact initializers and export the computed result"
            }
        }
    }
}

/// No values, generated text, loader state, or execution lives in this plan.
#[derive(Debug, Default)]
pub(crate) struct Selection {
    /// Source ordered; do not topologically reorder forward reads.
    pub units: Vec<Unit>,
    pub roots: Vec<Root>,
    pub imports: Vec<ImportBinding>,
    pub demands: Vec<ImportDemand>,
    pub reads: Vec<Read>,
    pub mutations: Vec<Mutation>,
    pub native_calls: Vec<NativeCall>,
    pub helper_calls: Vec<HelperCall>,
    /// Private native helper/alias units removable once their consumed roots
    /// disappear. An uncalled helper here must NEVER be executed eagerly.
    pub consumed: Vec<Unit>,
    pub escapes: Vec<Escape>,
    /// Conditional sites inside selected source, checked only if executed.
    pub checks: Vec<Escape>,
    /// All original lexical bindings, including excluded source and parameters.
    pub reserved_names: BTreeSet<String>,
}
