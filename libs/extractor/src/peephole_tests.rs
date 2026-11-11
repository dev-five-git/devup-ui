use super::*;
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

/// Parse `source`, mark every arrow it calls in place as generated, then simplify.
fn simplified(source: &str) -> String {
    struct Generated;
    impl<'a> VisitMut<'a> for Generated {
        fn visit_arrow_function_expression(
            &mut self,
            it: &mut oxc_ast::ast::ArrowFunctionExpression<'a>,
        ) {
            it.span = SPAN;
            walk_mut::walk_arrow_function_expression(self, it);
        }
    }
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, source, SourceType::jsx())
        .parse()
        .program;
    Generated.visit_program(&mut program);
    simplify(&AstBuilder::new(&allocator), &mut program);
    Codegen::new()
        .with_options(CodegenOptions {
            single_quote: false,
            ..CodegenOptions::default()
        })
        .build(&program)
        .code
}

#[rstest]
#[case::name_call("(0, f)(1);", "f(1);\n")]
#[case::direct_eval_kept("(0, eval)(s);", "(0, eval)(s);\n")]
#[case::method_kept("(0, a.b)();", "(0, a.b)();\n")]
#[case::optional_call_kept("(0, f)?.();", "(0, f)?.();\n")]
#[case::joined(r#"f(["a", "", "b"].filter(Boolean).join(" "));"#, "f(\"a b\");\n")]
#[case::joined_value_kept(
    r#"f(["a", x].filter(Boolean).join(" "));"#,
    "f([\"a\", x].filter(Boolean).join(\" \"));\n"
)]
#[case::joined_spread_kept(
    r#"f(["a", ...x].filter(Boolean).join(" "));"#,
    "f([\"a\", ...x].filter(Boolean).join(\" \"));\n"
)]
#[case::other_filter_kept(
    r#"f(["a"].filter(String).join(" "));"#,
    "f([\"a\"].filter(String).join(\" \"));\n"
)]
#[case::other_method_kept(
    r#"f(["a"].map(Boolean).join(" "));"#,
    "f([\"a\"].map(Boolean).join(\" \"));\n"
)]
#[case::other_join_kept(
    r#"f(["a"].filter(Boolean).concat(" "));"#,
    "f([\"a\"].filter(Boolean).concat(\" \"));\n"
)]
#[case::separator_kept(
    r#"f(["a"].filter(Boolean).join(s));"#,
    "f([\"a\"].filter(Boolean).join(s));\n"
)]
#[case::not_array_kept(
    r#"f(a.filter(Boolean).join(" "));"#,
    "f(a.filter(Boolean).join(\" \"));\n"
)]
#[case::not_filter_call_kept(r#"f(a.join(" "));"#, "f(a.join(\" \"));\n")]
#[case::computed_join_kept(
    r#"f(["a"].filter(Boolean)["join"](" "));"#,
    "f([\"a\"].filter(Boolean)[\"join\"](\" \"));\n"
)]
#[case::computed_filter_kept(
    r#"f(["a"]["filter"](Boolean).join(" "));"#,
    "f([\"a\"][\"filter\"](Boolean).join(\" \"));\n"
)]
#[case::optional_filter_kept(
    r#"f(["a"].filter?.(Boolean).join(" "));"#,
    "f([\"a\"].filter?.(Boolean).join(\" \"));\n"
)]
#[case::filter_value_kept(
    r#"f(["a"].filter(1).join(" "));"#,
    "f([\"a\"].filter(1).join(\" \"));\n"
)]
#[case::other_call_kept("f(g());", "f(g());\n")]
fn calls(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::text(r#"f("a" || x);"#, "f(\"a\");\n")]
#[case::template("f(`a${x}` || y);", "f(`a${x}`);\n")]
#[case::empty_template_kept("f(`${x}` || y);", "f(`${x}` || y);\n")]
#[case::number("f(1 || y);", "f(1);\n")]
#[case::zero_kept("f(0 || y);", "f(0 || y);\n")]
#[case::not_a_number_kept("f(NaN || y);", "f(NaN || y);\n")]
#[case::boolean("f(true || y);", "f(true);\n")]
#[case::object("f({} || y);", "f({});\n")]
#[case::function("f((() => 1) || y);", "f((() => 1));\n")]
#[case::joined_text(r#"f(x + " a" || y);"#, "f(x + \" a\");\n")]
#[case::joined_value_kept("f(x + y || z);", "f(x + y || z);\n")]
#[case::joined_empty_kept(r#"f(x + "" || z);"#, "f(x + \"\" || z);\n")]
#[case::subtraction_kept("f(x - 1 || z);", "f(x - 1 || z);\n")]
#[case::branches(r#"f((c ? "a" : "b") || y);"#, "f(c ? \"a\" : \"b\");\n")]
#[case::one_branch_kept(r#"f((c ? "a" : "") || y);"#, "f((c ? \"a\" : \"\") || y);\n")]
#[case::or_fallback(r#"f((x || "a") || y);"#, "f(x || \"a\");\n")]
#[case::coalesce_given(r#"f(("a" ?? x) || y);"#, "f(\"a\");\n")]
#[case::coalesce_kept(r#"f((x ?? "a") || y);"#, "f((x ?? \"a\") || y);\n")]
#[case::and(r#"f(("a" && "b") || y);"#, "f(\"a\" && \"b\");\n")]
#[case::and_kept(r#"f((x && "b") || y);"#, "f(x && \"b\" || y);\n")]
#[case::given_text(r#"f("" ?? y);"#, "f(\"\");\n")]
#[case::given_number("f(0 ?? y);", "f(0);\n")]
#[case::given_boolean("f(false ?? y);", "f(false);\n")]
#[case::given_template("f(`${x}` ?? y);", "f(`${x}`);\n")]
#[case::given_binary("f(x - 1 ?? y);", "f(x - 1);\n")]
#[case::given_not("f(!x ?? y);", "f(!x);\n")]
#[case::void_kept("f(void x ?? y);", "f(void x ?? y);\n")]
#[case::undefined_fallback("f(x ?? void 0);", "f(x);\n")]
#[case::unknown_kept("f(x ?? y);", "f(x ?? y);\n")]
#[case::and_kept_plain(r#"f("a" && y);"#, "f(\"a\" && y);\n")]
fn fallbacks(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::holds("f(true ? a : b);", "f(a);\n")]
#[case::fails("f(null ? a : b);", "f(b);\n")]
#[case::zero("f(0 ? a : b);", "f(b);\n")]
#[case::text(r#"f("x" ? a : b);"#, "f(a);\n")]
#[case::void("f(void 0 ? a : b);", "f(b);\n")]
#[case::void_text(r#"f(void "" ? a : b);"#, "f(b);\n")]
#[case::void_value_kept("f(void x ? a : b);", "f(void x ? a : b);\n")]
#[case::same_branches("f(__devupA ? x : x);", "f(x);\n")]
#[case::same_branches_authored_kept("f(c ? x : x);", "f(c ? x : x);\n")]
#[case::same_test_again(
    "f(__devupA ? __devupA ? a : b : __devupA ? c : d);",
    "f(__devupA ? a : d);\n"
)]
#[case::other_test_kept(
    "f(__devupA ? __devupB ? a : b : c);",
    "f(__devupA ? __devupB ? a : b : c);\n"
)]
#[case::boolean_test("f((c ? true : false) ? a : b);", "f(c ? a : b);\n")]
#[case::negated_test("f((c ? false : true) ? a : b);", "f(!c ? a : b);\n")]
#[case::and_test("f((c ? d : false) ? a : b);", "f(c && d ? a : b);\n")]
#[case::nested_and_test("f((c ? d ? false : true : false) ? a : b);", "f(c && !d ? a : b);\n")]
#[case::double_not("f(!!c ? a : b);", "f(c ? a : b);\n")]
#[case::not_inner("f(!(c ? true : false) ? a : b);", "f(!c ? a : b);\n")]
#[case::or_inner("f(x || (c ? true : false) ? a : b);", "f(x || c ? a : b);\n")]
#[case::branch_inner(
    "f((x ? (c ? true : false) : y) ? a : b);",
    "f((x ? c : y) ? a : b);\n"
)]
#[case::other_test_unchanged("f((x ? 1 : 2) ? a : b);", "f((x ? 1 : 2) ? a : b);\n")]
#[case::guarded(
    "const __devupG = g(); const __devupR = __devupG ? h() : undefined; f(__devupG && __devupR ? a : b);",
    "const __devupG = g();\nconst __devupR = __devupG ? h() : undefined;\nf(__devupR ? a : b);\n"
)]
#[case::guarded_void(
    "const __devupG = g(); const __devupR = __devupG ? h() : void 0; f(__devupG && __devupR ? a : b);",
    "const __devupG = g();\nconst __devupR = __devupG ? h() : void 0;\nf(__devupR ? a : b);\n"
)]
#[case::guarded_false(
    "const __devupG = g(); const __devupR = __devupG ? h() : false; f(__devupG && __devupR ? a : b);",
    "const __devupG = g();\nconst __devupR = __devupG ? h() : false;\nf(__devupR ? a : b);\n"
)]
#[case::other_guard_kept(
    "const __devupG = g(); const __devupR = __devupG ? h() : undefined; f(__devupH && __devupR ? a : b);",
    "const __devupG = g();\nconst __devupR = __devupG ? h() : undefined;\nf(__devupH && __devupR ? a : b);\n"
)]
#[case::reassigned_guard_kept(
    "let __devupG = g(); const __devupR = __devupG ? h() : undefined; __devupG = 0; f(__devupG && __devupR ? a : b);",
    "let __devupG = g();\nconst __devupR = __devupG ? h() : undefined;\n__devupG = 0;\nf(__devupG && __devupR ? a : b);\n"
)]
#[case::guard_with_value_kept(
    "const __devupG = g(); const __devupR = __devupG ? h() : 1; f(__devupG && __devupR ? a : b);",
    "const __devupG = g();\nconst __devupR = __devupG ? h() : 1;\nf(__devupG && __devupR ? a : b);\n"
)]
#[case::guard_by_value_kept(
    "const __devupR = g() ? h() : undefined; f(x && __devupR ? a : b);",
    "const __devupR = g() ? h() : undefined;\nf(x && __devupR ? a : b);\n"
)]
#[case::typeof_declared(
    r#"const on = g(); f(on ? typeof on === "string" ? on : "" : "");"#,
    "const on = g();\nf(typeof on === \"string\" ? on : \"\");\n"
)]
#[case::typeof_undeclared_kept(
    r#"f(on ? typeof on === "string" ? on : "" : "");"#,
    "f(on ? typeof on === \"string\" ? on : \"\" : \"\");\n"
)]
#[case::typeof_other_kind_kept(
    r#"const on = g(); f(on ? typeof on === "number" ? on : "" : "");"#,
    "const on = g();\nf(on ? typeof on === \"number\" ? on : \"\" : \"\");\n"
)]
#[case::typeof_other_value_kept(
    r#"const on = g(); f(on ? typeof on === "string" ? x : "" : "");"#,
    "const on = g();\nf(on ? typeof on === \"string\" ? x : \"\" : \"\");\n"
)]
#[case::typeof_other_fallback_kept(
    r#"const on = g(); f(on ? typeof on === "string" ? on : "" : "x");"#,
    "const on = g();\nf(on ? typeof on === \"string\" ? on : \"\" : \"x\");\n"
)]
fn conditionals(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::spliced("f(`a${`b${x}c`}d`);", "f(`ab${x}cd`);\n")]
#[case::text_only("f(`a${`b`}`);", "f(`ab`);\n")]
#[case::between("f(`${x}${`b`}${y}`);", "f(`${x}b${y}`);\n")]
#[case::tagged_inner_kept("f(`a${t`b`}`);", "f(`a${t`b`}`);\n")]
#[case::tagged_kept("t`a${`b`}`;", "t`a${`b`}`;\n")]
#[case::tag_inside_tagged("t`a${x || \"b\"}`;", "t`a${x || \"b\"}`;\n")]
#[case::invalid_escape_kept(r"t`a${t`\unicode`}`;", "t`a${t`\\unicode`}`;\n")]
fn templates(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::literal("f((1, `${x}`));", "f(`${x}`);\n")]
#[case::literals(r#"f((1, "a", null, true, g()));"#, "f(g());\n")]
#[case::compared("f((1 > 2, g()));", "f(g());\n")]
#[case::generated("f((__devupA, g()));", "f(g());\n")]
#[case::rest_kept("f((1, g(), x));", "f((g(), x));\n")]
#[case::read_kept("f((a, g()));", "f((a, g()));\n")]
#[case::compared_value_kept("f((a > 2, g()));", "f((a > 2, g()));\n")]
#[case::compared_to_value_kept("f((2 > a, g()));", "f((2 > a, g()));\n")]
#[case::membership_kept(r#"f(("a" in 1, g()));"#, "f((\"a\" in 1, g()));\n")]
#[case::instance_kept("f((1 instanceof 1, g()));", "f((1 instanceof 1, g()));\n")]
#[case::name_kept("typeof (1, x);", "typeof (1, x);\n")]
#[case::member_kept("(1, a.b)();", "(1, a.b)();\n")]
#[case::computed_member_kept("(1, a[b])();", "(1, a[b])();\n")]
#[case::private_member_kept(
    "class A { #p; m() { return (1, this.#p)(); } }",
    "class A {\n\t#p;\n\tm() {\n\t\treturn (1, this.#p)();\n\t}\n}\n"
)]
#[case::chain_kept("(1, a?.b)();", "(1, a?.b)();\n")]
#[case::function_kept("x = (1, function() {});", "x = (1, function() {});\n")]
#[case::arrow_kept("x = (1, () => 1);", "x = (1, () => 1);\n")]
#[case::class_kept("x = (1, class {});", "x = (1, class {});\n")]
fn sequences(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::spread_object("f({ a, ...{ b: 1, ...c } });", "f({\n\ta,\n\tb: 1,\n\t...c\n});\n")]
#[case::computed("f({ ...{ [k]: 1 } });", "f({ [k]: 1 });\n")]
#[case::proto_kept("f({ ...{ __proto__: null } });", "f({ ...{ __proto__: null } });\n")]
#[case::computed_proto(r#"f({ ...{ ["__proto__"]: 1 } });"#, "f({ [\"__proto__\"]: 1 });\n")]
#[case::getter_kept(
    "f({ ...{ get a() { return 1; } } });",
    "f({ ...{ get a() {\n\treturn 1;\n} } });\n"
)]
#[case::method_kept(
    "f({ ...{ a() { return 1; } } });",
    "f({ ...{ a() {\n\treturn 1;\n} } });\n"
)]
#[case::other_spread_kept("f({ ...x });", "f({ ...x });\n")]
fn objects(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::declared(
    "const __devupO = { a: 1 }; f(__devupO?.a, __devupO?.[k]);",
    "const __devupO = { a: 1 };\nf(__devupO.a, __devupO[k]);\n"
)]
#[case::array(
    "const __devupO = []; f(__devupO?.length);",
    "const __devupO = [];\nf(__devupO.length);\n"
)]
#[case::parameter(
    "f(((__devupO) => __devupO?.a)({}));",
    "f(((__devupO) => __devupO.a)({}));\n"
)]
#[case::defaulted_parameter_kept(
    "f(((__devupO = {}) => __devupO?.a)({}));",
    "f(((__devupO = {}) => __devupO?.a)({}));\n"
)]
#[case::rest_parameter_kept(
    "f(((__devupO, ...r) => __devupO?.a)({}));",
    "f(((__devupO, ...r) => __devupO?.a)({}));\n"
)]
#[case::optional_call_kept(
    "f(((__devupO) => __devupO?.a)?.({}));",
    "f(((__devupO) => __devupO?.a)?.({}));\n"
)]
#[case::authored_function_kept(
    "f((function(__devupO) { return __devupO?.a; })({}));",
    "f((function(__devupO) {\n\treturn __devupO?.a;\n})({}));\n"
)]
#[case::other_value_kept(
    "f(((__devupO) => __devupO?.a)(x));",
    "f(((__devupO) => __devupO?.a)(x));\n"
)]
#[case::rebound_kept(
    "const __devupO = {}; f((__devupO) => __devupO?.a);",
    "const __devupO = {};\nf((__devupO) => __devupO?.a);\n"
)]
#[case::written_kept(
    "let __devupO = {}; __devupO = null; f(__devupO?.a);",
    "let __devupO = {};\n__devupO = null;\nf(__devupO?.a);\n"
)]
#[case::written_by_pattern_kept(
    "let __devupO = {}; ({ __devupO } = x); f(__devupO?.a);",
    "let __devupO = {};\n({__devupO} = x);\nf(__devupO?.a);\n"
)]
#[case::authored_kept("const o = {}; f(o?.a);", "const o = {};\nf(o?.a);\n")]
#[case::let_kept(
    "let __devupO = {}; f(__devupO?.a);",
    "let __devupO = {};\nf(__devupO?.a);\n"
)]
#[case::destructured_kept(
    "const [__devupO] = [{}]; f(__devupO?.a);",
    "const [__devupO] = [{}];\nf(__devupO?.a);\n"
)]
fn objects_never_nullish(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::props(
    r#"<div {...{ a: 1, "b-c": "x", ...d }} />;"#,
    "<div a={1} b-c=\"x\" {...d} />;\n"
)]
#[case::quote_kept(r#"<div {...{ a: "\"" }} />;"#, "<div a={\"\\\"\"} />;\n")]
#[case::reference_kept(r#"<div {...{ a: "&amp;" }} />;"#, "<div a={\"&amp;\"} />;\n")]
#[case::backslash_kept(r#"<div {...{ a: "\\" }} />;"#, "<div a={\"\\\\\"} />;\n")]
#[case::line_kept(r#"<div {...{ a: "\n" }} />;"#, "<div a={\"\\n\"} />;\n")]
#[case::key_kept("<div {...{ key: 1 }} />;", "<div {...{ key: 1 }} />;\n")]
#[case::computed_kept("<div {...{ [k]: 1 }} />;", "<div {...{ [k]: 1 }} />;\n")]
#[case::number_key_kept("<div {...{ 1: 1 }} />;", "<div {...{ 1: 1 }} />;\n")]
#[case::dotted_key_kept(r#"<div {...{ "a.b": 1 }} />;"#, "<div {...{ \"a.b\": 1 }} />;\n")]
#[case::proto_kept(
    "<div {...{ __proto__: null }} />;",
    "<div {...{ __proto__: null }} />;\n"
)]
#[case::value_kept("<div {...x} />;", "<div {...x} />;\n")]
#[case::among_others(
    "<div a {...x} {...{ b: 1 }} {...{ key: 1 }} />;",
    "<div a {...x} b={1} {...{ key: 1 }} />;\n"
)]
fn jsx_spreads(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::elements(r#"f([("a"), (b), (c, d)]);"#, "f([\n\t\"a\",\n\tb,\n\t(c, d)\n]);\n")]
#[case::spread_kept("f([...(a)]);", "f([...a]);\n")]
#[case::hole_kept("f([, a]);", "f([, a]);\n")]
fn array_elements(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::read(
    "const __devupA = g(); const __devupB = __devupA; f(__devupB, { __devupB });",
    "const __devupA = g();\nf(__devupA, { __devupB: __devupA });\n"
)]
#[case::chain(
    "const __devupA = g(); const __devupB = __devupA; const __devupC = __devupB; f(__devupC);",
    "const __devupA = g();\nf(__devupA);\n"
)]
#[case::in_other_declarations(
    "const __devupA = g(), __devupB = __devupA, c = 1; f(__devupB, c);",
    "const __devupA = g(), c = 1;\nf(__devupA, c);\n"
)]
#[case::repeated_in_scopes(
    "function a() { const __devupA = g(); const __devupB = __devupA; return __devupB; } function b() { const __devupA = h(); const __devupB = __devupA; return __devupB; }",
    "function a() {\n\tconst __devupA = g();\n\treturn __devupA;\n}\nfunction b() {\n\tconst __devupA = h();\n\treturn __devupA;\n}\n"
)]
#[case::shadowed_kept(
    "const __devupA = 1; const __devupB = __devupA; f(() => { const __devupA = 2; return __devupB; });",
    "const __devupA = 1;\nconst __devupB = __devupA;\nf(() => {\n\tconst __devupA = 2;\n\treturn __devupB;\n});\n"
)]
#[case::written_kept(
    "let __devupA = g(); const __devupB = __devupA; __devupA = 1; f(__devupB);",
    "let __devupA = g();\nconst __devupB = __devupA;\n__devupA = 1;\nf(__devupB);\n"
)]
#[case::let_kept(
    "const __devupA = g(); let __devupB = __devupA; f(__devupB);",
    "const __devupA = g();\nlet __devupB = __devupA;\nf(__devupB);\n"
)]
#[case::authored_kept(
    "const __devupA = g(); const b = __devupA; f(b);",
    "const __devupA = g();\nconst b = __devupA;\nf(b);\n"
)]
#[case::authored_target_kept(
    "const a = g(); const __devupB = a; f(__devupB);",
    "const a = g();\nconst __devupB = a;\nf(__devupB);\n"
)]
#[case::global_kept(
    "const __devupB = __devupA; f(__devupB);",
    "const __devupB = __devupA;\nf(__devupB);\n"
)]
#[case::loop_kept(
    "const __devupA = g(); for (const __devupB = __devupA;;) f(__devupB);",
    "const __devupA = g();\nfor (const __devupB = __devupA;;) f(__devupA);\n"
)]
fn aliases(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}
