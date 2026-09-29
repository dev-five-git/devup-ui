//! Where a module changes the objects and arrays its top-level bindings hold,
//! or hands them to code that may change them, so the build does not read
//! them as the constants they are declared as.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, JSXElementName, Program, VariableDeclarationKind};
use oxc_semantic::{AstNodes, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::UnaryOperator;
use rustc_hash::FxHashMap;

/// How code uses a top-level binding
#[derive(Debug)]
pub(crate) enum Use {
    /// Changes what it holds, at this offset
    Changes(u32),
    /// Hands the value at `path` (`None` for any key) to code that may change
    /// it, or puts it in the top-level `const` named `into`
    Escapes {
        at: u32,
        path: Vec<Option<String>>,
        into: Option<String>,
    },
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

/// Global functions that only read their arguments
const READING_FUNCTIONS: [&str; 7] = [
    "String",
    "Number",
    "Boolean",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
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
    let context = Context {
        nodes,
        style,
        is_global: &is_global,
    };
    let mut uses: FxHashMap<String, Vec<Use>> = FxHashMap::default();
    for (name, symbol) in scoping.get_bindings(scoping.root_scope_id()) {
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
        let changes = |path: &[Option<String>]| (!path.is_empty()).then_some(Use::Changes(at));
        match kind {
            AstKind::AssignmentExpression(assignment) if assignment.left.span() == span => {
                changes(&path)
            }
            AstKind::AssignmentTargetWithDefault(target) if target.binding.span() == span => {
                changes(&path)
            }
            AstKind::ForInStatement(statement) if statement.left.span() == span => changes(&path),
            AstKind::ForOfStatement(statement) if statement.left.span() == span => changes(&path),
            AstKind::UpdateExpression(_)
            | AstKind::ArrayAssignmentTarget(_)
            | AstKind::ObjectAssignmentTarget(_)
            | AstKind::AssignmentTargetRest(_)
            | AstKind::AssignmentTargetPropertyProperty(_) => changes(&path),
            AstKind::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                changes(&path)
            }
            AstKind::CallExpression(call) if call.callee.span() == span => {
                matches!(path.last(), Some(Some(method)) if MUTATING_METHODS.contains(&method.as_str()))
                    .then_some(Use::Changes(at))
            }
            AstKind::CallExpression(call) => {
                let function = self.global_function(&call.callee);
                let changes_first = matches!(
                    function,
                    Some((
                        "Object",
                        "assign" | "defineProperty" | "defineProperties" | "setPrototypeOf"
                    ))
                );
                if changes_first && call.arguments.first().map(GetSpan::span) == Some(span) {
                    return Some(Use::Changes(at));
                }
                match function {
                    Some(("Object", "assign")) => {
                        path.push(None);
                        self.escapes(parent, at, path)
                    }
                    Some(
                        (
                            "Object",
                            "keys" | "values" | "entries" | "freeze" | "isFrozen"
                            | "getOwnPropertyNames" | "hasOwn",
                        )
                        | ("JSON" | "Math" | "console" | "", _)
                        | ("Array", "isArray"),
                    ) => None,
                    _ => self.escapes(parent, at, path),
                }
            }
            AstKind::SpreadElement(_) | AstKind::JSXSpreadAttribute(_) => {
                path.push(None);
                self.escapes(parent, at, path)
            }
            AstKind::ObjectProperty(property) if property.value.span() != span => None,
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
            | AstKind::JSXExpressionContainer(_)
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

    /// A value handed on at `node`: read where the style APIs read it, kept
    /// in a top-level `const` it is put in, or out of the build's sight
    fn escapes(&self, node: NodeId, at: u32, path: Vec<Option<String>>) -> Option<Use> {
        let ids = || std::iter::once(node).chain(self.nodes.ancestor_ids(node));
        let read = ids().any(|id| match self.nodes.kind(id) {
            AstKind::CallExpression(call) => self.is_style(&call.callee),
            AstKind::TaggedTemplateExpression(tagged) => self.is_style(&tagged.tag),
            AstKind::JSXOpeningElement(element) => self.is_style_element(&element.name),
            AstKind::JSXElement(element) => self.is_style_element(&element.opening_element.name),
            _ => false,
        });
        if read {
            return None;
        }
        // Through the literals it is written in, up to the declaration
        let declarator = ids().find(|id| {
            !matches!(
                self.nodes.kind(*id),
                AstKind::ObjectProperty(_)
                    | AstKind::ObjectExpression(_)
                    | AstKind::ArrayExpression(_)
                    | AstKind::SpreadElement(_)
                    | AstKind::ParenthesizedExpression(_)
                    | AstKind::TSAsExpression(_)
                    | AstKind::TSSatisfiesExpression(_)
            )
        });
        let into = declarator.and_then(|id| match self.nodes.kind(id) {
            AstKind::VariableDeclarator(declarator)
                if matches!(self.nodes.parent_kind(id),
                    AstKind::VariableDeclaration(declaration)
                        if declaration.kind == VariableDeclarationKind::Const)
                    && matches!(
                        self.nodes.ancestor_kinds(id).nth(1),
                        Some(AstKind::Program(_) | AstKind::ExportDeclaration(_))
                    ) =>
            {
                declarator
                    .id
                    .get_identifier_name()
                    .map(|name| name.to_string())
            }
            _ => None,
        });
        Some(Use::Escapes { at, path, into })
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
        let mut found: Vec<String> =
            uses(&program, &|name| matches!(name, "css" | "Box" | "Devup"))
                .into_iter()
                .flat_map(|(name, uses)| {
                    uses.into_iter().map(move |found| {
                        let describe = |at: u32| {
                            code[at as usize..]
                                .lines()
                                .next()
                                .unwrap_or_default()
                                .to_string()
                        };
                        match found {
                            Use::Changes(at) => format!("{name} changes: {}", describe(at)),
                            Use::Escapes { at, path, into } => {
                                format!("{name} escapes {path:?} into {into:?}: {}", describe(at))
                            }
                        }
                    })
                })
                .collect();
        found.sort();
        found
    }

    #[test]
    fn changes() {
        assert_eq!(
            describe(
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
a = 1;"
            ),
            [
                "a changes: a as any).x = 1;",
                "a changes: a!).x = 1;",
                "a changes: a, 'x', {});",
                "a changes: a, {});",
                "a changes: a.list.push(1);",
                "a changes: a.rest } = {});",
                "a changes: a.x = 1;",
                "a changes: a.x = 1] = [];",
                "a changes: a.x in {});",
                "a changes: a.x of []);",
                "a changes: a.x } = {});",
                "a changes: a.x;",
                "a changes: a.x] = [1];",
                "a changes: a?.x.sort();",
                "a changes: a['y'] += 1;",
                "a changes: a[k]++;",
            ]
        );
    }

    #[test]
    fn escapes() {
        assert_eq!(
            describe(
                "const a = { x: {} };
f(a.x);
new F(a);
Object.defineProperty(o, 'k', a);
Object.assign({}, a);
g(...a.list);
const copy = { ...a };
export const whole = { a, list: [a.x] };
const nested = f({ a });
let later = a;
later = a;
[later = a] = [];
function get() { return a; }
function* all() { yield a; }
const arrow = () => a.x;
const block = () => { a; };
const tagged = tag`${a}`;
<Other value={a} />;
<Other {...a} />;
<Other>{a}</Other>;
x ? a : a;
x && a;
(x, a);
await a;"
            ),
            [
                "a escapes [None] into None: a);",
                "a escapes [None] into None: a} />;",
                "a escapes [None] into Some(\"copy\"): a };",
                "a escapes [Some(\"list\"), None] into None: a.list);",
                "a escapes [Some(\"x\")] into None: a.x);",
                "a escapes [Some(\"x\")] into None: a.x;",
                "a escapes [Some(\"x\")] into Some(\"whole\"): a.x] };",
                "a escapes [] into None: a });",
                "a escapes [] into None: a);",
                "a escapes [] into None: a);",
                "a escapes [] into None: a;",
                "a escapes [] into None: a;",
                "a escapes [] into None: a; }",
                "a escapes [] into None: a; }",
                "a escapes [] into None: a] = [];",
                "a escapes [] into None: a} />;",
                "a escapes [] into None: a}</Other>;",
                "a escapes [] into None: a}`;",
                "a escapes [] into Some(\"whole\"): a, list: [a.x] };",
            ]
        );
    }

    #[test]
    fn reads() {
        assert_eq!(
            describe(
                "const a = { x: 1 };
String(a.x); parseInt(a.x); Object.keys(a); Object.hasOwn(a, 'x'); Object.getOwnPropertyNames(a); JSON.stringify(a); Math.max(a.x); console.log(a); Array.isArray(a);
css(a); css({ ...a, color: f(a) }); Devup.css(a); css.x`${a}`;
<Box {...a} p={f(a)} />; <Devup.Box>{a}</Devup.Box>;
const text = `${a.x}`; const sum = a.x + 1; const key = { [a.x]: 1 }; const b = { a: 1 }.a;
if (x ? 1 : 2) {} (a ? 1 : 2); (a, 1); a;
const other = a.x.toString(); [1][a.x]; typeof a; void a;
a.method(); o[a];
let unset; const { y = a } = {}; function p(q = a.x) {}
export default a; export { a as b };"
            ),
            [
                "a escapes [Some(\"x\")] into None: a.x) {}",
                "a escapes [] into None: a } = {}; function p(q = a.x) {}",
            ]
        );
    }

    #[test]
    fn globals_shadowed() {
        assert_eq!(
            describe(
                "const a = {};
const Object = { assign() {} };
const String = (x) => x;
Object.assign(a, {});
String(a);
(f.g)(a);
(0, h)(a);
(0, css)(a);
function inner() { const kept = { a }; }"
            ),
            [
                "a escapes [] into None: a }; }",
                "a escapes [] into None: a);",
                "a escapes [] into None: a);",
                "a escapes [] into None: a);",
                "a escapes [] into None: a);",
                "a escapes [] into None: a, {});",
            ]
        );
    }
}
