//! Where a module changes the objects and arrays its top-level bindings hold,
//! or hands them to code that may change them, so the build does not read
//! them as the constants they are declared as.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Expression, IdentifierReference, JSXAttributeName, JSXElementName, ObjectPropertyKind, Program,
    VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, Scoping, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashMap;

use crate::css_prop::{CssProp, CssTakers, binding_of};
use crate::imported_constants::jsx_root_identifier;
use crate::imported_constants::provenance::Proof;

pub(crate) mod callees;
mod compiled;
mod readonly_helpers;

/// How code uses a top-level binding
#[derive(Debug)]
pub(crate) enum Use {
    /// Changes what it holds `depth` members deep, at this offset
    Changes { at: u32, depth: usize },
    /// Hands the value at `path` (`None` for any key) to code that may change
    /// it, or puts it in the top-level `const` named `into`; an empty `into`
    /// is what the module exports
    Escapes {
        at: u32,
        path: Vec<Option<String>>,
        into: Option<String>,
    },
    /// Calls a method on the value at `path` that may change it through `this`
    Calls { at: u32, path: Vec<Option<String>> },
}

const MUTATING_METHODS: [&str; 13] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "sort",
    "reverse",
    "fill",
    "copyWithin",
    "set",
    "delete",
    "clear",
    "add",
];

/// Methods handing the elements of what they are called on to a callback or
/// to the array they return
const ELEMENT_METHODS: [&str; 23] = [
    "forEach",
    "map",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "some",
    "every",
    "reduce",
    "reduceRight",
    "flatMap",
    "flat",
    "concat",
    "slice",
    "toReversed",
    "toSorted",
    "toSpliced",
    "with",
    "values",
    "entries",
    "at",
    "get",
];

/// The uses of each top-level binding of `program` that may change what it
/// holds; `style` tells the names of the style APIs, whose arguments the
/// build reads and which never run, and `css` the `css` props the build reads
/// and the entry absorbing Emotion's own `jsx`. A name only counts for a
/// reference to the binding of the module, never to a local of the same name.
pub(crate) fn uses(
    program: &Program<'_>,
    style: &dyn Fn(&str) -> bool,
    css: Option<(CssProp, &str)>,
) -> FxHashMap<String, Vec<Use>> {
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let takers = css.map(|(css_prop, compat)| CssTakers::new(program, scoping, css_prop, compat));
    let mut uses: FxHashMap<String, Vec<Use>> = FxHashMap::default();
    for (name, symbol) in scoping.get_bindings(scoping.root_scope_id()) {
        let init = match nodes.kind(scoping.symbol_declaration(*symbol)) {
            AstKind::VariableDeclarator(declarator) => declarator.init.as_ref(),
            _ => None,
        };
        let context = Context {
            nodes,
            scoping,
            style,
            css: takers.as_ref(),
            init,
        };
        let found: Vec<Use> = scoping
            .get_resolved_reference_ids(*symbol)
            .iter()
            .map(|reference| scoping.get_reference(*reference))
            .filter(|reference| reference.is_value())
            .filter_map(|reference| context.classify(reference.node_id()))
            .collect();
        if !found.is_empty() {
            uses.insert(name.to_string(), found);
        }
    }
    uses
}

struct Context<'s, 'a> {
    nodes: &'s AstNodes<'a>,
    scoping: &'s Scoping,
    style: &'s dyn Fn(&str) -> bool,
    css: Option<&'s CssTakers<'s>>,
    /// What the binding is declared as, when a `const` or `let` gives it
    init: Option<&'s Expression<'a>>,
}

impl Context<'_, '_> {
    fn classify(&self, node: NodeId) -> Option<Use> {
        let at = self.nodes.kind(node).span().start;
        let mut path = Vec::new();
        let mut current = node;
        loop {
            let span = self.nodes.kind(current).span();
            let parent = self.nodes.parent_id(current);
            match self.nodes.kind(parent) {
                AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                    path.push(Some(member.property.name.to_string()));
                }
                AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                    path.push(
                        crate::utils::get_string_by_literal_expression(&member.expression)
                            .map(std::borrow::Cow::into_owned),
                    );
                }
                AstKind::ConditionalExpression(conditional) if conditional.test.span() == span => {
                    return None;
                }
                AstKind::SequenceExpression(sequence)
                    if sequence.expressions.last().map(GetSpan::span) != Some(span) =>
                {
                    return None;
                }
                AstKind::ParenthesizedExpression(_)
                | AstKind::TSAsExpression(_)
                | AstKind::TSSatisfiesExpression(_)
                | AstKind::TSNonNullExpression(_)
                | AstKind::TSTypeAssertion(_)
                | AstKind::ChainExpression(_)
                | AstKind::ConditionalExpression(_)
                | AstKind::LogicalExpression(_)
                | AstKind::SequenceExpression(_)
                | AstKind::AwaitExpression(_) => {}
                kind => return self.use_in(kind, parent, span, at, path),
            }
            current = parent;
        }
    }

    fn use_in(
        &self,
        kind: AstKind<'_>,
        parent: NodeId,
        span: oxc_span::Span,
        at: u32,
        mut path: Vec<Option<String>>,
    ) -> Option<Use> {
        // Writing the binding itself is an error for a `const` or an import
        let changes = |path: &[Option<String>]| {
            (!path.is_empty()).then_some(Use::Changes {
                at,
                depth: path.len(),
            })
        };
        match kind {
            AstKind::AssignmentExpression(assignment) if assignment.left.span() == span => {
                changes(&path)
            }
            AstKind::AssignmentTargetWithDefault(target) if target.binding.span() == span => {
                changes(&path)
            }
            AstKind::ForInStatement(statement) if statement.left.span() == span => changes(&path),
            AstKind::ForOfStatement(statement) if statement.left.span() == span => changes(&path),
            // Each element is handed to the loop's binding, or to what is spread
            AstKind::ForOfStatement(_) | AstKind::SpreadElement(_) => {
                path.push(None);
                self.escapes(parent, at, path)
            }
            AstKind::UpdateExpression(_)
            | AstKind::ArrayAssignmentTarget(_)
            | AstKind::ObjectAssignmentTarget(_)
            | AstKind::AssignmentTargetRest(_)
            | AstKind::AssignmentTargetPropertyProperty(_) => changes(&path),
            AstKind::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                changes(&path)
            }
            AstKind::CallExpression(call) if call.callee.span() == span => {
                self.method_call(parent, at, path)
            }
            AstKind::CallExpression(call) => {
                let function = self.global_function(&call.callee);
                if matches!(
                    function,
                    Some((
                        "Object",
                        "assign" | "defineProperty" | "defineProperties" | "setPrototypeOf"
                    ))
                ) && call.arguments.first().map(GetSpan::span) == Some(span)
                {
                    return Some(Use::Changes {
                        at,
                        depth: path.len() + 1,
                    });
                }
                match function {
                    Some(("Object", "freeze")) if call.arguments.len() == 1 => {
                        let found = self.classify(parent)?;
                        match found {
                            Use::Escapes { path: returned, into, .. } => {
                                path.extend(returned);
                                Some(Use::Escapes { at, path, into })
                            }
                            Use::Changes { depth, .. } => Some(Use::Changes { at, depth: path.len() + depth }),
                            Use::Calls { path: returned, .. } => {
                                path.extend(returned);
                                Some(Use::Calls { at, path })
                            }
                        }
                    }
                    Some(
                        ("Object", "assign" | "values" | "entries") | ("Array", "from" | "of"),
                    ) => {
                        path.push(None);
                        self.escapes(parent, at, path)
                    }
                    _ if callees::reads_arguments(&Proof { nodes: self.nodes, scoping: self.scoping }, call) => None,
                    _ => self.escapes(parent, at, path),
                }
            }
            AstKind::ObjectProperty(property) if property.value.span() != span => None,
            // React components must not change their props, so an element
            // only reads them, except the `ref` React assigns
            AstKind::JSXExpressionContainer(_) => {
                matches!(self.nodes.parent_kind(parent), AstKind::JSXAttribute(attribute)
                    if matches!(&attribute.name, JSXAttributeName::Identifier(name) if name.name == "ref"))
                .then(|| self.escapes(parent, at, path))
                .flatten()
            }
            AstKind::ObjectProperty(_)
            | AstKind::ArrayExpression(_)
            | AstKind::VariableDeclarator(_)
            | AstKind::AssignmentExpression(_)
            | AstKind::AssignmentTargetWithDefault(_)
            | AstKind::AssignmentPattern(_)
            | AstKind::FormalParameter(_)
            | AstKind::NewExpression(_)
            | AstKind::ReturnStatement(_)
            | AstKind::YieldExpression(_)
            | AstKind::ExportDefaultDeclaration(_)
            | AstKind::TaggedTemplateExpression(_)
            // The body of `() => value` gives its value
            | AstKind::ArrowFunctionExpression(_) => self.escapes(parent, at, path),
            AstKind::TemplateLiteral(_)
                if matches!(
                    self.nodes.parent_kind(parent),
                    AstKind::TaggedTemplateExpression(_)
                ) =>
            {
                self.escapes(parent, at, path)
            }
            _ => None,
        }
    }

    /// `path` read as a method called on what comes before its last key
    fn method_call(&self, call: NodeId, at: u32, mut path: Vec<Option<String>>) -> Option<Use> {
        // A local call can return captured references into a new holder.
        let Some(method) = path.pop() else {
            let AstKind::CallExpression(expression) = self.nodes.kind(call) else {
                return None;
            };
            let Expression::Identifier(identifier) = &expression.callee else {
                return None;
            };
            let symbol = binding_of(self.scoping, identifier)?;
            let proof = Proof {
                nodes: self.nodes,
                scoping: self.scoping,
            };
            let body = proof.factory(symbol)?;
            let mut captures = ReturnedCapture {
                proof: &proof,
                found: false,
            };
            captures.visit_expression(body);
            return captures.found.then(|| self.classify(call)).flatten();
        };
        if let Some(method) = method.as_deref() {
            if MUTATING_METHODS.contains(&method) {
                return Some(Use::Changes {
                    at,
                    depth: path.len() + 1,
                });
            }
            if ELEMENT_METHODS.contains(&method) {
                if !callees::pristine(
                    &Proof {
                        nodes: self.nodes,
                        scoping: self.scoping,
                    },
                    "Array",
                ) || !matches!(
                    self.init.map(|init| Proof {
                        nodes: self.nodes,
                        scoping: self.scoping
                    }
                    .expression(init)),
                    Some(crate::imported_constants::provenance::Shape::Array(_))
                ) {
                    return (!self.in_style(call)).then_some(Use::Calls { at, path });
                }
                path.push(None);
                return self.escapes(call, at, path);
            }
            if !self.uses_this(&path, method) {
                return None;
            }
        }
        (!self.in_style(call)).then_some(Use::Calls { at, path })
    }

    /// Whether the method `method` of what `path` leads to in the binding's
    /// declaration may read `this`: an arrow function cannot, and a function
    /// written there can only when its body does
    fn uses_this(&self, path: &[Option<String>], method: &str) -> bool {
        let mut value = self.init;
        for key in path.iter().map(Option::as_deref).chain([Some(method)]) {
            let (Some(key), Some(Expression::ObjectExpression(object))) =
                (key, value.map(crate::utils::unwrap_syntax_only))
            else {
                return true;
            };
            let mut found = None;
            for property in &object.properties {
                match property {
                    ObjectPropertyKind::ObjectProperty(property) => {
                        if property.key.static_name().as_deref() == Some(key) {
                            found = Some(&property.value);
                        } else if property.computed {
                            found = None;
                        }
                    }
                    ObjectPropertyKind::SpreadProperty(_) => found = None,
                }
            }
            value = found;
        }
        match value.map(crate::utils::unwrap_syntax_only) {
            Some(Expression::ArrowFunctionExpression(_)) => false,
            Some(Expression::FunctionExpression(function)) => {
                let mut this = ReadsThis::default();
                if let Some(body) = &function.body {
                    this.visit_function_body(body);
                }
                this.found
            }
            _ => true,
        }
    }

    /// The unmodified semantic global and its member, empty for a direct call.
    fn global_function<'e>(&self, callee: &'e Expression<'_>) -> Option<(&'e str, &'e str)> {
        callees::global(
            &Proof {
                nodes: self.nodes,
                scoping: self.scoping,
            },
            callee,
        )
    }

    /// Whether code at `node` is read by a style API, which never runs it
    fn in_style(&self, node: NodeId) -> bool {
        let mut runtime_call = false;
        std::iter::once(node)
            .chain(self.nodes.ancestor_ids(node))
            .find_map(|id| match self.nodes.kind(id) {
                AstKind::CallExpression(call) => {
                    if self.compiled_call(&call.callee)
                        || self.is_class_names_call(id, &call.callee)
                    {
                        Some(true)
                    } else {
                        runtime_call = true;
                        None
                    }
                }
                AstKind::TaggedTemplateExpression(tagged) => {
                    if self.compiled_call(&tagged.tag) || self.is_class_names_call(id, &tagged.tag)
                    {
                        Some(true)
                    } else {
                        runtime_call = true;
                        None
                    }
                }
                AstKind::JSXOpeningElement(element) => {
                    Some(!runtime_call && self.is_style_element(&element.name))
                }
                AstKind::JSXAttribute(attribute) if self.is_css_attribute(id, attribute) => {
                    Some(true)
                }
                AstKind::ObjectProperty(property) if self.is_css_property(id, property) => {
                    Some(true)
                }
                _ => None,
            })
            .unwrap_or(false)
    }

    /// Whether `callee`, called at `id`, is the `css` or `cx` a `<ClassNames>`
    /// child function around takes, whose calls the build compiles
    fn is_class_names_call(&self, id: NodeId, callee: &Expression<'_>) -> bool {
        let Some(css) = self.css else {
            return false;
        };
        self.nodes.ancestor_ids(id).any(|ancestor| {
            matches!(self.nodes.kind(ancestor), AstKind::JSXElement(element)
                if css.calls_class_names(&css.class_names_calls(element), callee))
        })
    }

    /// Whether the attribute `attribute` at `id` is a `css` prop the build
    /// compiles
    fn is_css_attribute(&self, id: NodeId, attribute: &oxc_ast::ast::JSXAttribute<'_>) -> bool {
        self.css.is_some_and(|css| {
            attribute
                .name
                .as_identifier()
                .is_some_and(|name| name.name == "css")
                && matches!(self.nodes.parent_kind(id), AstKind::JSXOpeningElement(element)
                    if css.takes(&element.name, |identifier| self.is_style_reference(identifier)))
        })
    }

    /// Whether the property `property` at `id` is the `css` prop among the
    /// props a `jsx()` call gives, which the build compiles
    fn is_css_property(&self, id: NodeId, property: &oxc_ast::ast::ObjectProperty<'_>) -> bool {
        self.css.is_some_and(|css| {
            matches!(self.nodes.parent_kind(self.nodes.parent_id(id)), AstKind::CallExpression(call)
                if matches!(call.arguments.get(1), Some(oxc_ast::ast::Argument::ObjectExpression(props))
                    if css.property(call, |identifier| self.is_style_reference(identifier))
                        .and_then(|at| props.properties.get(at))
                        .is_some_and(|css| css.span() == property.span)))
        })
    }

    /// A value handed on at `node`: read where the style APIs read it, kept
    /// in a top-level `const` or in what the module exports, or out of the
    /// build's sight
    fn escapes(&self, node: NodeId, at: u32, path: Vec<Option<String>>) -> Option<Use> {
        if self.in_style(node) {
            return None;
        }
        // Through the literals it is written in, and the calls giving back
        // what they are given, up to what holds it
        let holder = std::iter::once(node)
            .chain(self.nodes.ancestor_ids(node))
            .find(|id| match self.nodes.kind(*id) {
                AstKind::ObjectProperty(_)
                | AstKind::ObjectExpression(_)
                | AstKind::ArrayExpression(_)
                | AstKind::SpreadElement(_)
                | AstKind::ParenthesizedExpression(_)
                | AstKind::TSAsExpression(_)
                | AstKind::TSSatisfiesExpression(_)
                | AstKind::ArrowFunctionExpression(_)
                | AstKind::FunctionBody(_)
                | AstKind::ReturnStatement(_) => false,
                AstKind::CallExpression(call) => !matches!(
                    self.global_function(&call.callee),
                    Some(("Object", "freeze" | "seal" | "preventExtensions"))
                ),
                AstKind::Function(_) => matches!(
                    self.nodes.parent_kind(*id),
                    AstKind::Program(_) | AstKind::ExportDeclaration(_)
                ),
                _ => true,
            });
        let into = holder.and_then(|id| self.holder(id));
        Some(Use::Escapes { at, path, into })
    }

    /// The top-level `const` `id` declares, or an empty name for what the
    /// module exports at `id`
    fn holder(&self, id: NodeId) -> Option<String> {
        let top_level = |id: NodeId| {
            matches!(
                self.nodes.ancestor_kinds(id).nth(1),
                Some(AstKind::Program(_) | AstKind::ExportDeclaration(_))
            )
        };
        match self.nodes.kind(id) {
            AstKind::VariableDeclarator(declarator)
                if matches!(self.nodes.parent_kind(id),
                    AstKind::VariableDeclaration(declaration)
                        if declaration.kind == VariableDeclarationKind::Const)
                    && top_level(id) =>
            {
                declarator
                    .id
                    .get_identifier_name()
                    .map(|name| name.to_string())
            }
            AstKind::ExportDefaultDeclaration(_) => Some(String::new()),
            AstKind::Function(function)
                if matches!(
                    self.nodes.parent_kind(id),
                    AstKind::Program(_) | AstKind::ExportDeclaration(_)
                ) =>
            {
                function.id.as_ref().map(|id| id.name.to_string())
            }
            AstKind::AssignmentExpression(assignment)
                if top_level(id)
                    && assignment
                        .left
                        .as_simple_assignment_target()
                        .and_then(|target| target.as_member_expression())
                        .is_some_and(|member| self.is_commonjs_export(member)) =>
            {
                Some(String::new())
            }
            _ => None,
        }
    }

    /// `module.exports`, `module.exports.x` or `exports.x`
    fn is_commonjs_export(&self, member: &oxc_ast::ast::MemberExpression<'_>) -> bool {
        let is_global = |expression: &Expression<'_>, name: &str| {
            matches!(expression, Expression::Identifier(identifier)
                if identifier.name == name && self.is_global(identifier))
        };
        match member.object() {
            Expression::StaticMemberExpression(inner) => {
                inner.property.name == "exports" && is_global(&inner.object, "module")
            }
            object => {
                is_global(object, "exports")
                    || (is_global(object, "module")
                        && member.static_property_name() == Some("exports"))
            }
        }
    }

    /// Whether `identifier` reads a global, and not a local or an import of
    /// the module that shares the name
    fn is_global(&self, identifier: &IdentifierReference<'_>) -> bool {
        binding_of(self.scoping, identifier).is_none()
    }

    /// Whether `identifier` reads the top-level binding the style predicate
    /// names: not a local of the same name, and not a global
    fn is_style_reference(&self, identifier: &IdentifierReference<'_>) -> bool {
        (self.style)(&identifier.name)
            && binding_of(self.scoping, identifier).is_some_and(|symbol| {
                self.scoping
                    .get_root_binding(identifier.name.as_str().into())
                    == Some(symbol)
            })
    }

    fn is_style_element(&self, name: &JSXElementName<'_>) -> bool {
        jsx_root_identifier(name).is_some_and(|identifier| self.is_style_reference(identifier))
    }
}

/// Whether a function body reads `this`, outside the functions it declares
#[derive(Default)]
struct ReadsThis {
    found: bool,
    depth: usize,
}

struct ReturnedCapture<'s, 'a> {
    proof: &'s Proof<'s, 'a>,
    found: bool,
}

impl<'a> Visit<'a> for ReturnedCapture<'_, 'a> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if let Some(symbol) = binding_of(self.proof.scoping, identifier)
            && self.proof.scoping.symbol_scope_id(symbol) == self.proof.scoping.root_scope_id()
            && matches!(
                self.proof.binding(symbol),
                crate::imported_constants::provenance::Shape::Record(_)
                    | crate::imported_constants::provenance::Shape::Array(_)
                    | crate::imported_constants::provenance::Shape::Unknown
            )
        {
            self.found = true;
        }
    }
}

impl<'a> Visit<'a> for ReadsThis {
    fn visit_this_expression(&mut self, _: &oxc_ast::ast::ThisExpression) {
        self.found |= self.depth == 0;
    }

    fn visit_function(&mut self, function: &oxc_ast::ast::Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk::walk_function(self, function, flags);
        self.depth -= 1;
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    use super::{Use, uses};

    fn describe(code: &str) -> Vec<String> {
        let allocator = Allocator::default();
        let program = Parser::new(&allocator, code, SourceType::tsx())
            .parse()
            .program;
        let line = |at: u32| {
            code[at as usize..]
                .lines()
                .next()
                .unwrap_or_default()
                .to_string()
        };
        let mut found: Vec<String> = uses(
            &program,
            &|name| matches!(name, "css" | "Box" | "Devup"),
            None,
        )
        .into_iter()
        .flat_map(|(name, uses)| {
            uses.into_iter().map(move |found| match found {
                Use::Changes { at, depth } => {
                    format!("{name} changes {depth}: {}", line(at))
                }
                Use::Escapes { at, path, into } => {
                    format!("{name} escapes {path:?} into {into:?}: {}", line(at))
                }
                Use::Calls { at, path } => format!("{name} calls {path:?}: {}", line(at)),
            })
        })
        .collect();
        found.sort();
        found
    }

    #[test]
    fn changes() {
        insta::assert_debug_snapshot!(describe(
            "import { a } from './a';
a.x = 1;
a['y'] += 1;
a[k]++;
delete a.x;
[a.x] = [1];
({ x: a.x } = {});
({ ...a.rest } = {});
[a.x = 1] = [];
for (a.x in {});
for (a.x of []);
a.list.push(1);
a.list.map(String);
Object.assign(a, {});
Object.defineProperty(a, 'x', {});
(a as any).x = 1;
(a!).x = 1;
a?.x.sort();
a = 1;
a();"
        ));
    }

    #[test]
    fn escapes() {
        insta::assert_debug_snapshot!(describe(
            "const a = { x: {} };
f(a.x);
new F(a);
Object.defineProperty(o, 'k', a);
Object.assign({}, a);
Object.values(a); Object.entries(a); Array.from(a); Array.of(a);
g(...a.list);
const copy = { ...a };
export const whole = { a, list: [a.x] };
const frozen = Object.freeze({ a });
const nested = f({ a });
let later = a;
later = a;
[later = a] = [];
function get() { return a; }
function* all() { yield a; }
function p(q = a) {}
const arrow = () => a.x;
const block = () => { a; };
const tagged = tag`${a}`;
for (const item of a) {}
a.forEach(f); a.map(f);
export default { a };
module.exports = { a };
exports.b = a;
module.exports.c = a;
other.exports = a;
<div ref={a} />;
x ? a : a;
x && a;
(x, a);
await a;"
        ));
    }

    #[test]
    fn reads() {
        insta::assert_debug_snapshot!(describe(
            "import { css, Box } from '@devup-ui/react'; import * as Devup from '@devup-ui/react';
const a = { x: 1, arrow: () => 1, plain() { return 1; }, nested() { return function () { return this; }; }, self() { return this; } };
String(a.x); parseInt(a.x); structuredClone(a); Object.keys(a); Object.freeze(a); Object.hasOwn(a, 'x'); Object.getOwnPropertyNames(a); JSON.stringify(a); Math.max(a.x); console.log(a); Array.isArray(a);
css(a); css({ ...a, color: f(a) }); Devup.css(a); css.x`${a}`; css(1)(a); css({ w: a.self() });
<Box {...a} p={f(a)} />; <Devup.Box>{a}</Devup.Box>; <Other value={a} />; <Other {...a} />; <Other>{a}</Other>;
const text = `${a.x}`; const sum = a.x + 1; const key = { [a.x]: 1 }; const b = { a: 1 }.a;
if (x ? 1 : 2) {} (a ? 1 : 2); (a, 1); a; typeof a; void a;
a.x.toString(); a.join(','); [1][a.x]; o[a]; for (const k in a) {}
a.arrow(); a.plain(); a.nested(); a.self(); a[k](); a.missing();
let unset; const { y = a } = {};
export { a as b };"
        ));
    }

    #[test]
    fn this_through_the_declaration() {
        insta::assert_debug_snapshot!(describe(
            "const spread = { ...base, m() {} };
spread.m();
const computed = { [k]: 1, m() {} };
computed.m();
const path = { inner: { m() { return this; } } };
path.inner.m();
const indirect = { m: helper };
indirect.m();
const later = { m() {}, [k]: () => 1 };
later.m();
const deep = { x: 1 };
deep.x.y.m();
const plain = f();
plain.m();"
        ));
    }

    #[test]
    fn globals_shadowed() {
        insta::assert_debug_snapshot!(describe(
            "const a = {};
const Object = { assign() {} };
const String = (x) => x;
const exports = {};
Object.assign(a, {});
String(a);
(f.g)(a);
(0, h)(a);
(0, css)(a);
exports.x = a;
function inner() { const kept = { a }; }"
        ));
    }
}

#[cfg(test)]
mod scope_tests;
