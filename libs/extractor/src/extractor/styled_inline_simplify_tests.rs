use super::*;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;
use rstest::rstest;

fn simplified(source: &str, parameters: &[&str]) -> String {
    let arena = Allocator::default();
    let mut program = Parser::new(&arena, source, SourceType::tsx().with_module(false))
        .with_options(ParseOptions {
            allow_return_outside_function: true,
            ..ParseOptions::default()
        })
        .parse()
        .program;
    let ast = AstBuilder::new(&arena);
    simplify(&ast, &mut program.body, parameters);
    Codegen::new().build(&program).code
}

#[rstest]
#[case::flatten("let a = g(); { let b = a; return b; }", "const b = g();\nreturn b;\n")]
#[case::collision(
    "let a = g(); { let a = h(); { let c = a; return c; } }",
    "const a = g();\n{\n\tlet a = h();\n\tlet c = a;\n\treturn c;\n}\n"
)]
#[case::parameter("{ let p = g(); return p; }", "{\n\tlet p = g();\n\treturn p;\n}\n")]
#[case::read_before(
    "f(b); { let b = g(); return b; }",
    "f(b);\n{\n\tlet b = g();\n\treturn b;\n}\n"
)]
#[case::assign_once("let a; f(); a = g(); return a;", "f();\nconst a = g();\nreturn a;\n")]
#[case::assigned_twice(
    "let a; a = g(); a = h(); return a;",
    "let a;\na = g();\na = h();\nreturn a;\n"
)]
#[case::read_first(
    "let a; f(a); a = g(); return a;",
    "let a;\nf(a);\na = g();\nreturn a;\n"
)]
#[case::read_in_value("let a; a = g(a); return a;", "let a;\na = g(a);\nreturn a;\n")]
#[case::compound("let a; a += 1; return a;", "let a;\na += 1;\nreturn a;\n")]
#[case::unused("let a; return 1;", "let a;\nreturn 1;\n")]
#[case::omission(
    "let { x, ...rest } = p; let o; let d; return f(({ \"theme\": o, ...d } = rest, d));",
    "const { x, ...rest } = p;\nconst { \"theme\": o, ...d } = rest;\nreturn f(d);\n"
)]
#[case::omission_in_closure(
    "let { ...rest } = p; let o; let d; return () => ({ theme: o, ...d } = rest, d);",
    "const { ...rest } = p;\nlet o;\nlet d;\nreturn () => ({theme: o, ...d} = rest, d);\n"
)]
#[case::rest_read_twice(
    "let { ...rest } = p; let o; let d; g(rest); return ({ theme: o, ...d } = rest, d);",
    "const { ...rest } = p;\ng(rest);\nconst { theme: o, ...d } = rest;\nreturn d;\n"
)]
#[case::not_a_rest(
    "let rest = p; let o; let d; return ({ theme: o, ...d } = rest, d);",
    "const { theme: o, ...d } = p;\nreturn d;\n"
)]
#[case::other_read(
    "let { ...rest } = p; let o; let d; return ({ theme: o, ...d } = rest, o);",
    "const { ...rest } = p;\nconst { theme: o, ...d } = rest;\nreturn o;\n"
)]
#[case::computed_key(
    "let { ...rest } = p; let o; let d; return ({ [k]: o, ...d } = rest, d);",
    "const { ...rest } = p;\nlet o;\nlet d;\n({[k]: o, ...d} = rest);\nreturn d;\n"
)]
#[case::default_value(
    "let { ...rest } = p; let o; let d; return ({ theme: o = 1, ...d } = rest, d);",
    "const { ...rest } = p;\nlet o;\nlet d;\n({theme: o = 1, ...d} = rest);\nreturn d;\n"
)]
#[case::shorthand(
    "let { ...rest } = p; let o; let d; return ({ o, ...d } = rest, d);",
    "const { ...rest } = p;\nconst { o, ...d } = rest;\nreturn d;\n"
)]
#[case::not_a_slot(
    "let { ...rest } = p; let d; return ({ theme: o, ...d } = rest, d);",
    "const { ...rest } = p;\nlet d;\n({theme: o, ...d} = rest);\nreturn d;\n"
)]
#[case::no_rest(
    "let { ...rest } = p; let o; return ({ theme: o } = rest, o);",
    "const { ...rest } = p;\nconst { theme: o } = rest;\nreturn o;\n"
)]
#[case::member_rest(
    "let { ...rest } = p; let o; return ({ theme: o, ...q.d } = rest, q.d);",
    "const { ...rest } = p;\nlet o;\n({theme: o, ...q.d} = rest);\nreturn q.d;\n"
)]
#[case::array_target(
    "let { ...rest } = p; let o; return ([o] = rest, o);",
    "const { ...rest } = p;\nconst [o] = rest;\nreturn o;\n"
)]
#[case::member_source(
    "let o; let d; return ({ theme: o, ...d } = q.rest, d);",
    "const { theme: o, ...d } = q.rest;\nreturn d;\n"
)]
#[case::three_parts(
    "let { ...rest } = p; let o; let d; return ({ theme: o, ...d } = rest, f(), d);",
    "const { ...rest } = p;\nconst { theme: o, ...d } = rest;\nf();\nreturn d;\n"
)]
#[case::compound_source(
    "let { ...rest } = p; let d; return (d += rest, d);",
    "const { ...rest } = p;\nlet d;\nd += rest;\nreturn d;\n"
)]
#[case::empty_return("let a = g(); return;", "const a = g();\nreturn;\n")]
#[case::no_return("let a = g(); f(a);", "const a = g();\nf(a);\n")]
#[case::single_use_function(
    "let a = () => 1; let b = a; return b;",
    "const a = () => 1;\nconst b = a;\nreturn b;\n"
)]
#[case::single_use_pattern(
    "let a = () => 1; let { b } = a; return b;",
    "const { b } = () => 1;\nreturn b;\n"
)]
#[case::two_declarators(
    "let a = g(), c = h(); let b = a; return b + c;",
    "const a = g(), c = h();\nconst b = a;\nreturn b + c;\n"
)]
#[case::patterns(
    "let [a, , ...b] = p; let { c = 1, ...d } = q; return a + b + c + d;",
    "const [a, , ...b] = p;\nconst { c = 1, ...d } = q;\nreturn a + b + c + d;\n"
)]
#[case::array_written(
    "let [a] = p; [a] = q; for (const x of y) a++; return a;",
    "let [a] = p;\n[a] = q;\nfor (const x of y) a++;\nreturn a;\n"
)]
#[case::var_kept("var a = g(); return a;", "var a = g();\nreturn a;\n")]
#[case::split_declaration(
    "let r; let { a } = (r = f(), { a: r }); return a;",
    "const r = f();\nconst { a } = { a: r };\nreturn a;\n"
)]
#[case::split_nested(
    "let r; let a = (0, (r = f(), g(r))); return a;",
    "const r = f();\nconst a = g(r);\nreturn a;\n"
)]
#[case::split_keeps_unbound(
    "let a = (f(), () => 1); return a;",
    "f();\nconst a = (0, () => 1);\nreturn a;\n"
)]
#[case::split_keeps_directive_like("(f(), \"x\"); return 1;", "f(), \"x\";\nreturn 1;\n")]
#[case::split_object_pattern(
    "let a; let b; ({ k: a, j: [b] } = p); return a + b;",
    "const { k: a, j: [b] } = p;\nreturn a + b;\n"
)]
#[case::pattern_with_default(
    "let a; ([a = 1] = p); return a;",
    "let a;\n[a = 1] = p;\nreturn a;\n"
)]
#[case::pattern_read_before(
    "let a; f(a); ([a] = p); return a;",
    "let a;\nf(a);\n[a] = p;\nreturn a;\n"
)]
fn compiler_owned_renderers_shrink(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source, &["p"]), expected, "{source}");
}

#[rstest]
#[case::omitted("({ theme: o, ...d } = rest, d)", true)]
#[case::computed_key("({ [k]: o, ...d } = rest, d)", false)]
#[case::default_value("({ theme: o = 1, ...d } = rest, d)", false)]
#[case::shorthand("({ o, ...d } = rest, d)", false)]
#[case::not_a_slot("({ theme: x, ...d } = rest, d)", false)]
#[case::rest_not_a_slot("({ theme: o, ...x } = rest, x)", false)]
#[case::other_read("({ theme: o, ...d } = rest, o)", false)]
#[case::member_rest("({ theme: o, ...q.d } = rest, d)", false)]
#[case::no_rest("({ theme: o } = rest, d)", false)]
#[case::array_target("([o, ...d] = rest, d)", false)]
fn omissions_bind_only_slots_under_keys_written_as_is(#[case] source: &str, #[case] omitted: bool) {
    let arena = Allocator::default();
    let source = format!("let o; let d; f({source});");
    let program = Parser::new(&arena, &source, SourceType::mjs())
        .parse()
        .program;
    let Some(Statement::ExpressionStatement(statement)) = program.body.last() else {
        panic!("expected a call")
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("expected a call")
    };
    let value = call.arguments[0]
        .as_expression()
        .unwrap_or_else(|| panic!("expected a value"));
    let rests = FxHashSet::from_iter(["rest".to_string()]);
    let slots = FxHashSet::from_iter(["o".to_string(), "d".to_string()]);
    let uses = Uses::of(&program.body);
    let ast = AstBuilder::new(&arena);
    assert_eq!(
        omission(&ast, unwrap_syntax_only(value), &rests, &slots, &uses).is_some(),
        omitted,
        "{source}"
    );
}
