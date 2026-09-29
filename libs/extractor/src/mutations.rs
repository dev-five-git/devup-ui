//! Where a module changes the objects and arrays its top-level bindings hold,
//! or hands them to code that may change them, so the build does not read
//! them as the constants they are declared as.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Expression, JSXAttributeName, JSXElementName, ObjectPropertyKind, Program,
    VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashMap;

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

/// Methods that only read what they are called on
const READING_METHODS: [&str; 10] = [
    "join",
    "includes",
    "indexOf",
    "lastIndexOf",
    "keys",
    "has",
    "toString",
    "valueOf",
    "hasOwnProperty",
    "propertyIsEnumerable",
];

/// Global functions that only read their arguments
const READING_FUNCTIONS: [&str; 8] = [
    "String",
    "Number",
    "Boolean",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "structuredClone",
];

/// The uses of each top-level binding of `program` that may change what it
/// holds; `style` tells the names of the style APIs, whose arguments the
/// build reads and which never run
pub(crate) fn uses(
    program: &Program<'_>,
    style: &dyn Fn(&str) -> bool,
) -> FxHashMap<String, Vec<Use>> {
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let is_global = |name: &str| scoping.get_root_binding(name.into()).is_none();
    let mut uses: FxHashMap<String, Vec<Use>> = FxHashMap::default();
    for (name, symbol) in scoping.get_bindings(scoping.root_scope_id()) {
        let init = match nodes.kind(scoping.symbol_declaration(*symbol)) {
            AstKind::VariableDeclarator(declarator) => declarator.init.as_ref(),
            _ => None,
        };
        let context = Context {
            nodes,
            style,
            is_global: &is_global,
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
    style: &'s dyn Fn(&str) -> bool,
    is_global: &'s dyn Fn(&str) -> bool,
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
                    Some(
                        ("Object", "assign" | "values" | "entries") | ("Array", "from" | "of"),
                    ) => {
                        path.push(None);
                        self.escapes(parent, at, path)
                    }
                    Some(
                        (
                            "Object",
                            "keys" | "freeze" | "seal" | "preventExtensions" | "isFrozen"
                            | "isSealed" | "getOwnPropertyNames" | "hasOwn",
                        )
                        | ("JSON" | "Math" | "console" | "", _)
                        | ("Array", "isArray"),
                    ) => None,
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
        // Calling the binding itself hands it nothing
        let method = path.pop()?;
        if let Some(method) = method.as_deref() {
            if MUTATING_METHODS.contains(&method) {
                return Some(Use::Changes {
                    at,
                    depth: path.len() + 1,
                });
            }
            if ELEMENT_METHODS.contains(&method) {
                path.push(None);
                return self.escapes(call, at, path);
            }
            if READING_METHODS.contains(&method) || !self.uses_this(&path, method) {
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

    /// `(object, member)` when `callee` is a global function, `("", name)` for
    /// one that only reads its arguments
    fn global_function<'e>(&self, callee: &'e Expression<'_>) -> Option<(&'e str, &'e str)> {
        match callee {
            Expression::Identifier(identifier)
                if READING_FUNCTIONS.contains(&identifier.name.as_str())
                    && (self.is_global)(&identifier.name) =>
            {
                Some(("", identifier.name.as_str()))
            }
            Expression::StaticMemberExpression(member) => match &member.object {
                Expression::Identifier(object) if (self.is_global)(&object.name) => {
                    Some((object.name.as_str(), member.property.name.as_str()))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether code at `node` is read by a style API, which never runs it
    fn in_style(&self, node: NodeId) -> bool {
        std::iter::once(node)
            .chain(self.nodes.ancestor_ids(node))
            .any(|id| match self.nodes.kind(id) {
                AstKind::CallExpression(call) => self.is_style(&call.callee),
                AstKind::TaggedTemplateExpression(tagged) => self.is_style(&tagged.tag),
                AstKind::JSXOpeningElement(element) => self.is_style_element(&element.name),
                _ => false,
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
                | AstKind::TSSatisfiesExpression(_) => false,
                AstKind::CallExpression(call) => !matches!(
                    self.global_function(&call.callee),
                    Some(("Object", "freeze" | "seal" | "preventExtensions"))
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
                if identifier.name == name && (self.is_global)(name))
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

    fn is_style(&self, callee: &Expression<'_>) -> bool {
        let mut expression = callee;
        loop {
            match crate::utils::unwrap_syntax_only(expression) {
                Expression::Identifier(identifier) => return (self.style)(&identifier.name),
                Expression::StaticMemberExpression(member) => expression = &member.object,
                Expression::CallExpression(call) => expression = &call.callee,
                _ => return false,
            }
        }
    }

    fn is_style_element(&self, name: &JSXElementName<'_>) -> bool {
        crate::imported_constants::jsx_root(name).is_some_and(|root| (self.style)(root))
    }
}

/// Whether a function body reads `this`, outside the functions it declares
#[derive(Default)]
struct ReadsThis {
    found: bool,
    depth: usize,
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
        let mut found: Vec<String> =
            uses(&program, &|name| matches!(name, "css" | "Box" | "Devup"))
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
            "const a = { x: 1, arrow: () => 1, plain() { return 1; }, nested() { return function () { return this; }; }, self() { return this; } };
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
