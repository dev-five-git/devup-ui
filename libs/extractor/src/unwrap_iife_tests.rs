use super::*;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

/// Parse `source`, mark every arrow it writes `__g` at as generated, then unwrap.
fn unwrapped(source: &str) -> String {
    unwrap_with(source, SourceType::mjs())
}

fn unwrapped_jsx(source: &str) -> String {
    unwrap_with(source, SourceType::jsx())
}

fn unwrap_with(source: &str, source_type: SourceType) -> String {
    struct Generated;
    impl<'a> VisitMut<'a> for Generated {
        fn visit_arrow_function_expression(&mut self, it: &mut ArrowFunctionExpression<'a>) {
            if it
                .params
                .items
                .first()
                .and_then(|parameter| parameter.pattern.get_binding_identifier())
                .is_some_and(|id| id.name.starts_with("__g"))
                || it.params.items.is_empty()
            {
                it.span = SPAN;
            }
            walk_mut::walk_arrow_function_expression(self, it);
        }
        fn visit_variable_declaration(&mut self, it: &mut oxc_ast::ast::VariableDeclaration<'a>) {
            if it.declarations.iter().any(|declarator| {
                declarator
                    .id
                    .get_binding_identifier()
                    .is_some_and(|id| id.name.starts_with("__devup"))
            }) {
                it.span = SPAN;
            }
            walk_mut::walk_variable_declaration(self, it);
        }
    }
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, source, source_type).parse().program;
    Generated.visit_program(&mut program);
    unwrap(&AstBuilder::new(&allocator), &mut program);
    Codegen::new().build(&program).code
}

#[rstest]
#[case::statement("((__g0) => f(__g0))(a());", "const __g0 = a();\nf(__g0);\n")]
#[case::returned(
    "function c() { return ((__g0, __g1) => [__g0, __g1])(a(), b()); }",
    "function c() {\n\tconst __g0 = a(), __g1 = b();\n\treturn [__g0, __g1];\n}\n"
)]
#[case::declared(
    "const x = ((__g0) => () => __g0)(a());",
    "const __g0 = a();\nconst x = (0, () => __g0);\n"
)]
#[case::argument_named("((__g0) => __g0)(() => 1);", "const __g0 = (0, () => 1);\n__g0;\n")]
#[case::block_body(
    "{ (() => { const __g0 = a(); return f(__g0); })(); }",
    "{\n\tconst __g0 = a();\n\tf(__g0);\n}\n"
)]
#[case::arrow_body(
    "const c = (p) => ((__g0) => f(__g0, p))(a());",
    "const c = (p) => {\n\tconst __g0 = a();\n\treturn f(__g0, p);\n};\n"
)]
#[case::nested(
    "((__g0) => ((__g1) => f(__g0, __g1))(b()))(a());",
    "const __g0 = a();\nconst __g1 = b();\nf(__g0, __g1);\n"
)]
#[case::catch(
    "try {} catch (__g0) { ((__g0) => f(__g0))(a()); }",
    "try {} catch (__g0) {\n\t((__g0) => f(__g0))(a());\n}\n"
)]
#[case::static_block(
    "class C { static { ((__g0) => f(__g0))(a()); } }",
    "class C {\n\tstatic {\n\t\tconst __g0 = a();\n\t\tf(__g0);\n\t}\n}\n"
)]
#[case::parameter(
    "function c(__g0) { return ((__g0) => f(__g0))(a()); }",
    "function c(__g0) {\n\treturn ((__g0) => f(__g0))(a());\n}\n"
)]
#[case::argument_reads_name("((__g0) => f(__g0))(__g0);", "((__g0) => f(__g0))(__g0);\n")]
#[case::sibling_name(
    "((__g0) => f(__g0))(a()); ((__g0) => g(__g0))(b());",
    "((__g0) => f(__g0))(a());\n((__g0) => g(__g0))(b());\n"
)]
#[case::directive("((__g0) => \"x\")(a());", "((__g0) => \"x\")(a());\n")]
#[case::eval(
    "((__g0) => f(__g0))(a()); eval('');",
    "((__g0) => f(__g0))(a());\neval(\"\");\n"
)]
#[case::authored("((a) => f(a))(b());", "((a) => f(a))(b());\n")]
#[case::not_head("x || ((__g0) => f(__g0))(a());", "x || ((__g0) => f(__g0))(a());\n")]
#[case::optional("((__g0) => f(__g0))?.(a());", "((__g0) => f(__g0))?.(a());\n")]
#[case::spread("((__g0) => f(__g0))(...a);", "((__g0) => f(__g0))(...a);\n")]
#[case::fewer_arguments("((__g0) => f(__g0))();", "((__g0) => f(__g0))();\n")]
#[case::asynchronous("(async (__g0) => f(__g0))(a());", "(async (__g0) => f(__g0))(a());\n")]
#[case::rest("((__g0, ...r) => f(__g0))(a());", "((__g0, ...r) => f(__g0))(a());\n")]
#[case::default("((__g0 = 1) => f(__g0))(a());", "((__g0 = 1) => f(__g0))(a());\n")]
#[case::pattern("(([__g0]) => f(__g0))(a());", "(([__g0]) => f(__g0))(a());\n")]
#[case::let_body(
    "(() => { let __g0 = a(); return f(__g0); })();",
    "(() => {\n\tlet __g0 = a();\n\treturn f(__g0);\n})();\n"
)]
#[case::statement_body(
    "(() => { a(); return f(); })();",
    "(() => {\n\ta();\n\treturn f();\n})();\n"
)]
#[case::pattern_body(
    "(() => { const [__g0] = a(); return f(__g0); })();",
    "(() => {\n\tconst [__g0] = a();\n\treturn f(__g0);\n})();\n"
)]
#[case::empty_return("(() => { return; })();", "(() => {\n\treturn;\n})();\n")]
#[case::no_return("(() => {})();", "(() => {})();\n")]
#[case::directive_body(
    "(() => { 'use strict'; return f(); })();",
    "(() => {\n\t\"use strict\";\n\treturn f();\n})();\n"
)]
#[case::many_declarators(
    "let a = 1, b = ((__g0) => __g0)(c());",
    "let a = 1, b = ((__g0) => __g0)(c());\n"
)]
fn generated_calls_unwrap_where_a_statement_evaluates_them_first(
    #[case] source: &str,
    #[case] expected: &str,
) {
    assert_eq!(unwrapped(source), expected, "{source}");
}

#[rstest]
#[case::unused(
    "function f() { const __devupA = []; return 1; }",
    "function f() {
	return 1;
}
"
)]
#[case::unused_function(
    "function f() { const __devupA = () => g(); return 1; }",
    "function f() {
	return 1;
}
"
)]
#[case::unused_call(
    "function f() { const __devupA = g(); return 1; }",
    "function f() {
	const __devupA = g();
	return 1;
}
"
)]
#[case::authored(
    "function f() { const a = []; return 1; }",
    "function f() {
	const a = [];
	return 1;
}
"
)]
#[case::parameter(
    "function f(p) { const __devupA = p; return g(__devupA, { __devupA }); }",
    "function f(p) {
	return g(p, { __devupA: p });
}
"
)]
#[case::import(
    "import { D } from 'm'; function f() { const __devupA = D; return <__devupA />; }",
    "import { D } from \"m\";
function f() {
	return <D />;
}
"
)]
#[case::import_shadowed(
    "import { D } from 'm'; function f(D) { const __devupA = D; return <__devupA />; }",
    "import { D } from \"m\";
function f(D) {
	return <D />;
}
"
)]
#[case::lowercase_element(
    "function f(d) { const __devupA = d; return <__devupA />; }",
    "function f(d) {
	const __devupA = d;
	return <__devupA />;
}
"
)]
#[case::written(
    "function f(p) { const __devupA = p; p = 1; return __devupA; }",
    "function f(p) {
	const __devupA = p;
	p = 1;
	return __devupA;
}
"
)]
#[case::rebound(
    "function f(p) { const __devupA = p; return () => { const p = 1; return __devupA; }; }",
    "function f(p) {
	const __devupA = p;
	return () => {
		const p = 1;
		return __devupA;
	};
}
"
)]
#[case::arguments(
    "function f(p) { const __devupA = p; arguments[0] = 1; return __devupA; }",
    "function f(p) {
	const __devupA = p;
	arguments[0] = 1;
	return __devupA;
}
"
)]
#[case::read_before(
    "function f(p) { const g = () => __devupA; const __devupA = p; return g; }",
    "function f(p) {
	const g = () => __devupA;
	const __devupA = p;
	return g;
}
"
)]
#[case::exported(
    "const __devupA = 1; export { __devupA };",
    "const __devupA = 1;
export { __devupA };
"
)]
#[case::arrow_returning(
    "const f = (__g0) => { return __g0; };",
    "const f = (__g0) => __g0;
"
)]
fn generated_declarations_go_where_nothing_needs_them(
    #[case] source: &str,
    #[case] expected: &str,
) {
    assert_eq!(unwrapped_jsx(source), expected, "{source}");
}

#[rstest]
#[case::text(r#"f(`a${"px"}b${x}`);"#, "f(`apxb${x}`);\n")]
#[case::whole(r#"f(`${"px"}`);"#, "f(`px`);\n")]
#[case::escaped(r#"f(`${"`"}${"${"}${"\\"}${"$"}`);"#, "f(`\\`\\${\\\\$`);\n")]
#[case::carriage_return(r#"f(`${"a\rb"}`);"#, "f(`a\\rb`);\n")]
#[case::parenthesized(r#"f(`${("px")}`);"#, "f(`px`);\n")]
#[case::other_value("f(`${1}${x}`);", "f(`${1}${x}`);\n")]
#[case::invalid_escape(r#"t(`\u{}${"px"}`);"#, "t(`\\u{}${\"px\"}`);\n")]
#[case::tagged(r#"t`a${"px"}${`${"in"}`}`;"#, "t`a${\"px\"}${`in`}`;\n")]
fn strings_in_template_literals_are_written_as_their_text(
    #[case] source: &str,
    #[case] expected: &str,
) {
    assert_eq!(unwrapped(source), expected, "{source}");
}
