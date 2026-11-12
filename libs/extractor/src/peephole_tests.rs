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
#[case::falsy_zero("f(0 || y);", "f(y);\n")]
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
#[case::same_branches("const x = 1; f(__devupA ? x : x);", "const x = 1;\nf(x);\n")]
#[case::same_branches_undeclared("f(__devupA ? x : x);", "f((__devupA, x));\n")]
#[case::same_branches_authored("f(c ? x : x);", "f((c, x));\n")]
#[case::same_branches_compared(
    "function c(a, b) { return a === b ? `${a}` : `${a}`; }",
    "function c(a, b) {\n\treturn `${a}`;\n}\n"
)]
#[case::same_branches_constants_kept(
    "const a = 1, b = 2; f(a === b ? `${a}` : `${a}`);",
    "const a = 1, b = 2;\nf((a === b, `${a}`));\n"
)]
#[case::same_branches_class_import_kept(
    "import a from \"m\"; class b {} f(a === b ? `${a}` : `${a}`);",
    "import a from \"m\";\nclass b {}\nf((a === b, `${a}`));\n"
)]
#[case::same_branches_loosely_compared_kept(
    "function c(a, b) { return a == b ? `${a}` : `${a}`; }",
    "function c(a, b) {\n\treturn a == b, `${a}`;\n}\n"
)]
#[case::same_elements("b ? <div /> : <div />;", "b, <div />;\n")]
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
#[case::shared(
    r#"import a from "a"; const x = Object.fromEntries(Object.entries(p).filter(([k]) => k[0] !== "$")); const y = Object.fromEntries(Object.entries(q).filter(([k]) => k[0] !== "$"));"#,
    "import a from \"a\";\nfunction __devupOmit0(__devupOmitted) {\n\treturn Object.fromEntries(Object.entries(__devupOmitted).filter(([k]) => k[0] !== \"$\"));\n}\nconst x = __devupOmit0(p);\nconst y = __devupOmit0(q);\n"
)]
#[case::once_kept(
    r#"const x = Object.fromEntries(Object.entries(p).filter(([k]) => k[0] !== "$")); const y = Object.fromEntries(Object.entries(q).filter(([k]) => k !== "a"));"#,
    "const x = Object.fromEntries(Object.entries(p).filter(([k]) => k[0] !== \"$\"));\nconst y = Object.fromEntries(Object.entries(q).filter(([k]) => k !== \"a\"));\n"
)]
#[case::name_taken(
    r"const __devupOmit0 = 1; f(Object.fromEntries(Object.entries(p).filter(([k]) => k)), Object.fromEntries(Object.entries(q).filter(([k]) => k)));",
    "function __devupOmit1(__devupOmitted) {\n\treturn Object.fromEntries(Object.entries(__devupOmitted).filter(([k]) => k));\n}\nconst __devupOmit0 = 1;\nf(__devupOmit1(p), __devupOmit1(q));\n"
)]
#[case::free_name_kept(
    r"f(Object.fromEntries(Object.entries(p).filter(([k]) => k !== n)), Object.fromEntries(Object.entries(q).filter(([k]) => k !== n)));",
    "f(Object.fromEntries(Object.entries(p).filter(([k]) => k !== n)), Object.fromEntries(Object.entries(q).filter(([k]) => k !== n)));\n"
)]
#[case::object_declared_kept(
    r"const Object = g(); f(Object.fromEntries(Object.entries(p).filter(([k]) => k)), Object.fromEntries(Object.entries(q).filter(([k]) => k)));",
    "const Object = g();\nf(Object.fromEntries(Object.entries(p).filter(([k]) => k)), Object.fromEntries(Object.entries(q).filter(([k]) => k)));\n"
)]
#[case::other_shapes_kept(
    r"f(Object.fromEntries(Object.entries(p).filter((k) => k)), Object.fromEntries(Object.entries(q).filter((k) => k)), Object.fromEntries(Object.entries(p).map(([k]) => k)), Object.fromEntries(Object.entries(q).map(([k]) => k)), Object.fromEntries(Object.entries(p, 1).filter(([k]) => k)), Object.fromEntries(Object.entries(q, 1).filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter(async ([k]) => k)), Object.fromEntries(Object.entries(q).filter(async ([k]) => k)), Object.fromEntries(Object.entries(p).filter(([k], ...r) => k)), Object.fromEntries(Object.entries(q).filter(([k], ...r) => k)), Object.fromEntries(Object.entries(p).filter(([k, ...r]) => k)), Object.fromEntries(Object.entries(q).filter(([k, ...r]) => k)), Object.fromEntries(Object.entries(p).filter(([k] = 1) => k)), Object.fromEntries(Object.entries(q).filter(([k] = 1) => k)), Object.fromEntries(Object.entries(p).filter(([k]) => this)), Object.fromEntries(Object.entries(q).filter(([k]) => this)), Object?.fromEntries(Object.entries(p).filter(([k]) => k)), Object?.fromEntries(Object.entries(q).filter(([k]) => k)), Object.fromEntries(Object.entries(p)?.filter(([k]) => k)), Object.fromEntries(Object.entries(q)?.filter(([k]) => k)), Object.fromEntries(...a), Object.fromEntries(...b));",
    "f(Object.fromEntries(Object.entries(p).filter((k) => k)), Object.fromEntries(Object.entries(q).filter((k) => k)), Object.fromEntries(Object.entries(p).map(([k]) => k)), Object.fromEntries(Object.entries(q).map(([k]) => k)), Object.fromEntries(Object.entries(p, 1).filter(([k]) => k)), Object.fromEntries(Object.entries(q, 1).filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter(async ([k]) => k)), Object.fromEntries(Object.entries(q).filter(async ([k]) => k)), Object.fromEntries(Object.entries(p).filter(([k], ...r) => k)), Object.fromEntries(Object.entries(q).filter(([k], ...r) => k)), Object.fromEntries(Object.entries(p).filter(([k, ...r]) => k)), Object.fromEntries(Object.entries(q).filter(([k, ...r]) => k)), Object.fromEntries(Object.entries(p).filter(([k] = 1) => k)), Object.fromEntries(Object.entries(q).filter(([k] = 1) => k)), Object.fromEntries(Object.entries(p).filter(([k]) => this)), Object.fromEntries(Object.entries(q).filter(([k]) => this)), Object?.fromEntries(Object.entries(p).filter(([k]) => k)), Object?.fromEntries(Object.entries(q).filter(([k]) => k)), Object.fromEntries(Object.entries(p)?.filter(([k]) => k)), Object.fromEntries(Object.entries(q)?.filter(([k]) => k)), Object.fromEntries(...a), Object.fromEntries(...b));\n"
)]
#[case::shared_beside_one(
    "f(Object.fromEntries(Object.entries(p).filter(([k]) => k)), Object.fromEntries(Object.entries(q).filter(([k]) => k)), Object.fromEntries(Object.entries(r).filter(([k]) => !k)));",
    "function __devupOmit0(__devupOmitted) {\n\treturn Object.fromEntries(Object.entries(__devupOmitted).filter(([k]) => k));\n}\nf(__devupOmit0(p), __devupOmit0(q), Object.fromEntries(Object.entries(r).filter(([k]) => !k)));\n"
)]
#[case::parenthesized(
    r"f(Object.fromEntries((Object.entries(p)).filter(([k]) => k)), Object.fromEntries(Object.entries(q).filter(([k]) => k)));",
    "function __devupOmit0(__devupOmitted) {\n\treturn Object.fromEntries(Object.entries(__devupOmitted).filter(([k]) => k));\n}\nf(__devupOmit0(p), __devupOmit0(q));\n"
)]
#[case::more_shapes_kept(
    r#"f(Object.fromEntries(Object.entries(...p).filter(([k]) => k)), Object.fromEntries(Object.entries(...q).filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter(...g)), Object.fromEntries(Object.entries(q).filter(...g)), Object.fromEntries(Object.entries(p)["filter"](([k]) => k)), Object.fromEntries(Object.entries(q)["filter"](([k]) => k)), Object.fromEntries(Object.entries(p).filter(([k]) => k, 1)), Object.fromEntries(Object.entries(q).filter(([k]) => k, 1)), Object.fromEntries(e.filter(([k]) => k)), Object.fromEntries(e.filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter(g)), Object.fromEntries(Object.entries(q).filter(g)), Object.fromEntries(Object.entries(p).filter(([[k]]) => k)), Object.fromEntries(Object.entries(q).filter(([[k]]) => k)), Object.fromEntries(Object.entries(p).filter(([k, v]) => k)), Object.fromEntries(Object.entries(q).filter(([k, v]) => k)), Object.fromEntries(Object.entries(p).filter(({ k }) => k)), Object.fromEntries(Object.entries(q).filter(({ k }) => k)), Object.fromEntries(Object.entries(p).filter(() => 1)), Object.fromEntries(Object.entries(q).filter(() => 1)), Object.fromEntries(Object.entries?.(p).filter(([k]) => k)), Object.fromEntries(Object.entries?.(q).filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter?.(([k]) => k)), Object.fromEntries(Object.entries(q).filter?.(([k]) => k)), Object.fromEntries?.(Object.entries(p).filter(([k]) => k)), Object.fromEntries?.(Object.entries(q).filter(([k]) => k)), O.fromEntries(Object.entries(p).filter(([k]) => k)), O.fromEntries(Object.entries(q).filter(([k]) => k)));"#,
    "f(Object.fromEntries(Object.entries(...p).filter(([k]) => k)), Object.fromEntries(Object.entries(...q).filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter(...g)), Object.fromEntries(Object.entries(q).filter(...g)), Object.fromEntries(Object.entries(p)[\"filter\"](([k]) => k)), Object.fromEntries(Object.entries(q)[\"filter\"](([k]) => k)), Object.fromEntries(Object.entries(p).filter(([k]) => k, 1)), Object.fromEntries(Object.entries(q).filter(([k]) => k, 1)), Object.fromEntries(e.filter(([k]) => k)), Object.fromEntries(e.filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter(g)), Object.fromEntries(Object.entries(q).filter(g)), Object.fromEntries(Object.entries(p).filter(([[k]]) => k)), Object.fromEntries(Object.entries(q).filter(([[k]]) => k)), Object.fromEntries(Object.entries(p).filter(([k, v]) => k)), Object.fromEntries(Object.entries(q).filter(([k, v]) => k)), Object.fromEntries(Object.entries(p).filter(({ k }) => k)), Object.fromEntries(Object.entries(q).filter(({ k }) => k)), Object.fromEntries(Object.entries(p).filter(() => 1)), Object.fromEntries(Object.entries(q).filter(() => 1)), Object.fromEntries(Object.entries?.(p).filter(([k]) => k)), Object.fromEntries(Object.entries?.(q).filter(([k]) => k)), Object.fromEntries(Object.entries(p).filter?.(([k]) => k)), Object.fromEntries(Object.entries(q).filter?.(([k]) => k)), Object.fromEntries?.(Object.entries(p).filter(([k]) => k)), Object.fromEntries?.(Object.entries(q).filter(([k]) => k)), O.fromEntries(Object.entries(p).filter(([k]) => k)), O.fromEntries(Object.entries(q).filter(([k]) => k)));\n"
)]
fn shares_filters(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::guarded(
    "const __devupB = c ? `${c}px` : void 0; <div className={__devupB ? \"a\" : \"\"} />;",
    "const __devupB = c ? `${c}px` : void 0;\n<div className={__devupB && \"a\"} />;\n"
)]
#[case::component_kept(
    "const __devupB = c ? `${c}px` : void 0; <A className={__devupB ? \"a\" : \"\"} />;",
    "const __devupB = c ? `${c}px` : void 0;\n<A className={__devupB ? \"a\" : \"\"} />;\n"
)]
#[case::maybe_empty_kept(
    "const __devupB = c ? `${c}` : void 0; <div className={__devupB ? \"a\" : \"\"} />;",
    "const __devupB = c ? `${c}` : void 0;\n<div className={__devupB ? \"a\" : \"\"} />;\n"
)]
#[case::false_kept(
    "const __devupB = c ? \"x\" : false; <div className={__devupB ? \"a\" : \"\"} />;",
    "const __devupB = c ? \"x\" : false;\n<div className={__devupB ? \"a\" : \"\"} />;\n"
)]
#[case::boolean_test_kept(
    "<div className={a === b ? \"a\" : \"\"} />;",
    "<div className={a === b ? \"a\" : \"\"} />;\n"
)]
#[case::other_attribute_kept(
    "const __devupB = c ? `${c}px` : void 0; <div title={__devupB ? \"a\" : \"\"} />;",
    "const __devupB = c ? `${c}px` : void 0;\n<div title={__devupB ? \"a\" : \"\"} />;\n"
)]
#[case::other_fallback_kept(
    "const __devupB = c ? `${c}px` : void 0; <div className={__devupB ? \"a\" : \"b\"} id=\"x\" {...p} />;",
    "const __devupB = c ? `${c}px` : void 0;\n<div className={__devupB ? \"a\" : \"b\"} id=\"x\" {...p} />;\n"
)]
#[case::assigned_kept(
    "let __devupB = c ? `${c}px` : void 0; __devupB = 0; <div className={__devupB ? \"a\" : \"\"} />;",
    "let __devupB = c ? `${c}px` : void 0;\n__devupB = 0;\n<div className={__devupB ? \"a\" : \"\"} />;\n"
)]
fn class_or_none(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::attrs(
    "function c(__devupP, __devupR) { const __devupProps = { ...__devupP, ref: __devupR }; const { a, ...rest } = { ...__devupProps, role: \"note\" }; return rest; }",
    "function c(__devupP, __devupR) {\n\tconst { a, ...rest } = __devupP;\n\trest.ref = __devupR;\n\trest.role = \"note\";\n\treturn rest;\n}\n"
)]
#[case::attrs_in_arrow(
    "f((__devupP, __devupR) => { const __devupProps = { ...__devupP, ref: __devupR }; const { a, ...rest } = { ...__devupProps, type: \"text\", title: \"t\", type: \"password\" }; return <b {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f(({ a, ...rest }, __devupR) => <b {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} ref={__devupR} type=\"password\" title=\"t\" />);\n"
)]
#[case::read_again_kept(
    "const __devupProps = { ...p }; const { ...rest } = { ...__devupProps, a: __devupProps.a };",
    "const __devupProps = { ...p };\nconst { ...rest } = {\n\t...__devupProps,\n\ta: __devupProps.a\n};\n"
)]
#[case::not_first_kept(
    "const __devupProps = { ...p }; const { ...rest } = { a: 1, ...__devupProps };",
    "const __devupProps = { ...p };\nconst { ...rest } = {\n\ta: 1,\n\t...__devupProps\n};\n"
)]
#[case::authored_kept(
    "const props = { ...p }; const { ...rest } = { ...props, a: 1 };",
    "const props = { ...p };\nconst { ...rest } = {\n\t...props,\n\ta: 1\n};\n"
)]
#[case::let_kept(
    "let __devupProps = { ...p }; const { ...rest } = { ...__devupProps, a: 1 };",
    "let __devupProps = { ...p };\nconst { ...rest } = __devupProps;\nrest.a = 1;\n"
)]
#[case::not_spread_next_kept(
    "const __devupProps = { ...p }; f(__devupProps); const { ...rest } = { a: 1 };",
    "const __devupProps = { ...p };\nf(__devupProps);\nconst { ...rest } = { a: 1 };\n"
)]
#[case::spread_other_kept(
    "const __devupProps = { ...p }; const { ...rest } = { ...q, a: __devupProps };",
    "const __devupProps = { ...p };\nconst { ...rest } = {\n\t...q,\n\ta: __devupProps\n};\n"
)]
#[case::whole_value(
    "const __devupProps = { ...p }; const { ...rest } = __devupProps;",
    "const { ...rest } = { ...p };\n"
)]
#[case::named_function_kept(
    "const __devupF = () => 1; const f = __devupF;",
    "const __devupF = () => 1;\nconst f = __devupF;\n"
)]
#[case::not_set_next_kept(
    "const __devupA = g(); let b; f(__devupA);",
    "const __devupA = g();\nlet b;\nf(__devupA);\n"
)]
#[case::two_declarators_kept(
    "const __devupProps = { ...p }, b = 1; const { ...rest } = { ...__devupProps }; const c = 1, d = { ...e };",
    "const __devupProps = { ...p }, b = 1;\nconst { ...rest } = { ...__devupProps };\nconst c = 1, d = { ...e };\n"
)]
#[case::pattern_kept(
    "const { __devupProps } = { ...p }; const { ...rest } = { ...__devupProps };",
    "const { __devupProps } = { ...p };\nconst { ...rest } = { ...__devupProps };\n"
)]
#[case::two_declarators_next_kept(
    "const __devupA = g(); const b = __devupA, c = 1;",
    "const __devupA = g();\nconst b = __devupA, c = 1;\n"
)]
fn spreads_inline(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::filtered(
    "f((__devupP, __devupR) => { const { style, as: A = \"div\", ...rest } = __devupP; rest.ref = __devupR; return <A {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\" && k !== \"theme\"))} style={style} />; });",
    "f(({ style, as: A = \"div\", ...rest }, __devupR) => <A {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\" && k !== \"theme\"))} ref={__devupR} style={style} />);\n"
)]
#[case::kept_by_equality(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => !(k === \"x\")))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => !(k === \"x\")))} />;\n});\n"
)]
#[case::ref_withheld_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k !== \"ref\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k !== \"ref\"))} />;\n});\n"
)]
#[case::first_letter_r_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] === \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] === \"$\"))} />;\n});\n"
)]
#[case::rest_read_again_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k))} b={rest} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k))} b={rest} />;\n});\n"
)]
#[case::props_read_again_kept(
    "f((__devupP, __devupR) => { const { a = __devupP.b, ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { a = __devupP.b, ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::ref_read_again(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} b={__devupR} />; });",
    "f(({ ...rest }, __devupR) => <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} ref={__devupR} b={__devupR} />);\n"
)]
#[case::values(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; rest[\"aria-label\"] = \"x\"; rest.a = 1; rest.b = \"q\\\"\"; rest.a = 2; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f(({ ...rest }, __devupR) => <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} ref={__devupR} aria-label=\"x\" a={2} b={\"q\\\"\"} />);\n"
)]
#[case::not_assigned_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; if (c) {} return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\tif (c) {}\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::called_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; g(); return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\tg();\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::name_assigned_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; __devupX = 1; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\t__devupX = 1;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::other_object_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; q.a = 1; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\tq.a = 1;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::not_attribute_name_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest[\"a b\"] = 1; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest[\"a b\"] = 1;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::running_value_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.a = g(); return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.a = g();\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::attribute_written_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.b = 1; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} b={2} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.b = 1;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} b={2} />;\n});\n"
)]
#[case::nothing_assigned_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::not_filtered_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...rest} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...rest} />;\n});\n"
)]
#[case::other_assignment_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.key = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.key = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::compound_assignment_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref += __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref += __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::other_value(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = 1; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} b={__devupR} />; });",
    "f(({ ...rest }, __devupR) => <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} ref={1} b={__devupR} />);\n"
)]
#[case::let_kept(
    "f((__devupP, __devupR) => { let { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\tlet { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::other_init_kept(
    "f((__devupP, __devupR) => { const { ...rest } = q; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} b={__devupP} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = q;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} b={__devupP} />;\n});\n"
)]
#[case::authored_kept(
    "f((p, r) => { const { ...rest } = p; rest.ref = r; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((p, r) => {\n\tconst { ...rest } = p;\n\trest.ref = r;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::strict_kept(
    "f((__devupP, __devupR) => { \"use strict\"; const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; });",
    "f((__devupP, __devupR) => {\n\t\"use strict\";\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n});\n"
)]
#[case::shapes_kept(
    "f((__devupP) => 1, (__devupP, __devupR, ...__devupS) => { return 1; }, ({ a }, __devupR) => { return 1; }, (__devupP, __devupR) => 1, (__devupP, __devupR) => { return 1; }, (__devupP, __devupR) => { const a = 1, b = 2; f(); return <a />; }, (__devupP, __devupR) => { const [a] = __devupP; f(); return <a />; }, (__devupP, __devupR) => { const { a } = __devupP; f(); return <a />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; f(); return <a />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return 1; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest[r] = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k.length))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[1] !== \"$\"))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k !== n))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter((k) => k))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([, k]) => k))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => { return k; }))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(q).filter(([k]) => k[0] !== \"#\"))} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...g(rest)} />; }, (__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return; });",
    "f((__devupP) => 1, (__devupP, __devupR, ...__devupS) => {\n\treturn 1;\n}, ({ a }, __devupR) => {\n\treturn 1;\n}, (__devupP, __devupR) => 1, (__devupP, __devupR) => {\n\treturn 1;\n}, (__devupP, __devupR) => {\n\tconst a = 1, b = 2;\n\tf();\n\treturn <a />;\n}, (__devupP, __devupR) => {\n\tconst [a] = __devupP;\n\tf();\n\treturn <a />;\n}, (__devupP, __devupR) => {\n\tconst { a } = __devupP;\n\tf();\n\treturn <a />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\tf();\n\treturn <a />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn 1;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest[r] = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[0] !== \"$\"))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k.length))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k[1] !== \"$\"))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k !== n))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter((k) => k))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([, k]) => k))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => {\n\t\treturn k;\n\t}))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(q).filter(([k]) => k[0] !== \"#\"))} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...g(rest)} />;\n}, (__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn;\n});\n"
)]
#[case::loosely_compared_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k != \"x\"))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k != \"x\"))} />;\n});\n"
)]
#[case::compared_to_name_kept(
    "f((__devupP, __devupR) => { const { ...rest } = __devupP; rest.ref = __devupR; return <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k !== k))} />; });",
    "f((__devupP, __devupR) => {\n\tconst { ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn <a {...Object.fromEntries(Object.entries(rest).filter(([k]) => k !== k))} />;\n});\n"
)]
fn ref_as_attribute(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::rest(
    "function c(__devupP, __devupR) { const { a, b: d = 1, ...rest } = { ...__devupP, ref: __devupR }; return rest; }",
    "function c(__devupP, __devupR) {\n\tconst { a, b: d = 1, ...rest } = __devupP;\n\trest.ref = __devupR;\n\treturn rest;\n}\n"
)]
#[case::named_ref_kept(
    "const { ref: r, ...rest } = { ...__devupP, ref: __devupR };",
    "const { ref: r, ...rest } = {\n\t...__devupP,\n\tref: __devupR\n};\n"
)]
#[case::computed_kept(
    "const { [k]: r, ...rest } = { ...__devupP, ref: __devupR };",
    "const { [k]: r, ...rest } = {\n\t...__devupP,\n\tref: __devupR\n};\n"
)]
#[case::no_rest_kept(
    "const { a } = { ...__devupP, ref: __devupR };",
    "const { a } = {\n\t...__devupP,\n\tref: __devupR\n};\n"
)]
#[case::authored_props_kept(
    "const { ...rest } = { ...p, ref: __devupR };",
    "const { ...rest } = {\n\t...p,\n\tref: __devupR\n};\n"
)]
#[case::props(
    "const { a, ...rest } = { ...__devupP, ref: __devupR, role: \"note\", \"data-x\": 1 };",
    "const { a, ...rest } = __devupP;\nrest.ref = __devupR;\nrest.role = \"note\";\nrest[\"data-x\"] = 1;\n"
)]
#[case::named_prop_kept(
    "const { role, ...rest } = { ...__devupP, role: \"note\" };",
    "const { role, ...rest } = {\n\t...__devupP,\n\trole: \"note\"\n};\n"
)]
#[case::running_value_kept(
    "const { ...rest } = { ...__devupP, role: g() };",
    "const { ...rest } = {\n\t...__devupP,\n\trole: g()\n};\n"
)]
#[case::spread_after_kept(
    "const { ...rest } = { ...__devupP, ...__devupQ };",
    "const { ...rest } = {\n\t...__devupP,\n\t...__devupQ\n};\n"
)]
#[case::proto_kept(
    "const { ...rest } = { ...__devupP, __proto__: null };",
    "const { ...rest } = {\n\t...__devupP,\n\t__proto__: null\n};\n"
)]
#[case::getter_kept(
    "const { ...rest } = { ...__devupP, get ref() { return 1; } };",
    "const { ...rest } = {\n\t...__devupP,\n\tget ref() {\n\t\treturn 1;\n\t}\n};\n"
)]
#[case::not_copied_kept("const { ...rest } = __devupP;", "const { ...rest } = __devupP;\n")]
#[case::spread_alone_kept(
    "const { ...rest } = { ...__devupP };",
    "const { ...rest } = { ...__devupP };\n"
)]
fn rest_takes_ref(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::after(
    r#"const { className } = p; f(["a b", className].filter(Boolean).join(" "));"#,
    "const { className } = p;\nf(className ? `a b ${className}` : \"a b\");\n"
)]
#[case::between(
    r#"f(["a", "", __devupA, "b", "c"].filter(Boolean).join("-"));"#,
    "f(__devupA ? `a-${__devupA}-b-c` : \"a-b-c\");\n"
)]
#[case::alone(
    r#"f([__devupA].filter(Boolean).join(" "));"#,
    "f(__devupA ? `${__devupA}` : \"\");\n"
)]
#[case::undeclared_kept(
    r#"f(["a", x].filter(Boolean).join(" "));"#,
    "f([\"a\", x].filter(Boolean).join(\" \"));\n"
)]
#[case::two_names_kept(
    r#"f([__devupA, __devupB].filter(Boolean).join(" "));"#,
    "f([__devupA, __devupB].filter(Boolean).join(\" \"));\n"
)]
#[case::member_kept(
    r#"f(["a", __devupA.b].filter(Boolean).join(" "));"#,
    "f([\"a\", __devupA.b].filter(Boolean).join(\" \"));\n"
)]
#[case::escaped_kept(
    r#"f(["a$", __devupA].filter(Boolean).join(" "));"#,
    "f([\"a$\", __devupA].filter(Boolean).join(\" \"));\n"
)]
fn joins_around_a_name(#[case] source: &str, #[case] expected: &str) {
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
    "const __devupA = 1; g(); const __devupB = __devupA; f(() => { const __devupA = 2; return __devupB; });",
    "const __devupA = 1;\ng();\nconst __devupB = __devupA;\nf(() => {\n\tconst __devupA = 2;\n\treturn __devupB;\n});\n"
)]
#[case::written_kept(
    "let __devupA = g(); const __devupB = __devupA; __devupA = 1; f(__devupB);",
    "let __devupA = g();\nconst __devupB = __devupA;\n__devupA = 1;\nf(__devupB);\n"
)]
#[case::let_kept(
    "const __devupA = g(); h(); let __devupB = __devupA; f(__devupB);",
    "const __devupA = g();\nh();\nlet __devupB = __devupA;\nf(__devupB);\n"
)]
#[case::authored_kept(
    "const __devupA = g(); h(); const b = __devupA; f(b);",
    "const __devupA = g();\nh();\nconst b = __devupA;\nf(b);\n"
)]
#[case::constant_set_before(
    "const a = g(); const __devupB = a; f(() => __devupB);",
    "const a = g();\nf(() => a);\n"
)]
#[case::constant_of_outer_statements_kept(
    "const a = g(); function c() { const __devupB = a; return () => __devupB; }",
    "const a = g();\nfunction c() {\n\tconst __devupB = a;\n\treturn () => __devupB;\n}\n"
)]
#[case::constant_set_after_kept(
    "function c() { const __devupB = a; return () => __devupB; } const a = g();",
    "function c() {\n\tconst __devupB = a;\n\treturn () => __devupB;\n}\nconst a = g();\n"
)]
#[case::import_kept(
    "import { a } from \"m\"; const __devupB = a; f(__devupB);",
    "import { a } from \"m\";\nconst __devupB = a;\nf(__devupB);\n"
)]
#[case::let_set_before(
    "let a = g(); const __devupB = a; f(__devupB);",
    "let a = g();\nf(a);\n"
)]
#[case::var_kept(
    "var a = g(); const __devupB = a; f(__devupB);",
    "var a = g();\nconst __devupB = a;\nf(__devupB);\n"
)]
#[case::parameter(
    "function c(a) { const __devupB = a; return () => __devupB; }",
    "function c(a) {\n\treturn () => a;\n}\n"
)]
#[case::function(
    "function a() {} const __devupB = a; f(__devupB);",
    "function a() {}\nf(a);\n"
)]
#[case::constant_same_statement(
    "const a = g(), __devupB = a; f(__devupB);",
    "const a = g();\nf(a);\n"
)]
fn aliases(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::literals(
    "const __devupA = { a: 1, b: \"x\" }; f(__devupA.a, __devupA.c, __devupA.toString);",
    "const __devupA = {\n\ta: 1,\n\tb: \"x\"\n};\nf(1, void 0, __devupA.toString);\n"
)]
#[case::through_spreads(
    "const __devupA = { ...p, a: 1 }; const __devupB = { ...__devupA, b: 2 }; f(__devupB.a, __devupB.c);",
    "const __devupB = {\n\t...p,\n\ta: 1,\n\tb: 2\n};\nf(1, __devupB.c);\n"
)]
#[case::assigned_kept(
    "const __devupA = { a: 1 }; __devupA.a = 2; f(__devupA.a);",
    "const __devupA = { a: 1 };\n__devupA.a = 2;\nf(__devupA.a);\n"
)]
#[case::method_called_kept(
    "const __devupA = { a: 1 }; __devupA.m(); f(__devupA.a);",
    "const __devupA = { a: 1 };\n__devupA.m();\nf(__devupA.a);\n"
)]
#[case::deleted_kept(
    "const __devupA = { a: 1 }; delete __devupA.a; f(__devupA.a);",
    "const __devupA = { a: 1 };\ndelete __devupA.a;\nf(__devupA.a);\n"
)]
#[case::read_as_value_kept(
    "const __devupA = { a: 1 }; g(__devupA); f(__devupA.a);",
    "const __devupA = { a: 1 };\ng(__devupA);\nf(__devupA.a);\n"
)]
#[case::shadowed_source(
    "const __devupA = { ...p }; const __devupB = { ...__devupA }; function g() { const __devupA = 1; return __devupB.x; }",
    "const __devupB = { ...p };\nfunction g() {\n\tconst __devupA = 1;\n\treturn __devupB.x;\n}\n"
)]
#[case::proto_kept(
    "const __devupA = { __proto__: q, a: 1 }; f(__devupA.a);",
    "const __devupA = {\n\t__proto__: q,\n\ta: 1\n};\nf(__devupA.a);\n"
)]
#[case::getter_kept(
    "const __devupA = { get a() { return 1; } }; f(__devupA.a);",
    "const __devupA = { get a() {\n\treturn 1;\n} };\nf(__devupA.a);\n"
)]
#[case::computed(
    "const __devupA = { [k]: 1, a: 2 }; f(__devupA.a, __devupA.b);",
    "const __devupA = {\n\t[k]: 1,\n\ta: 2\n};\nf(2, __devupA.b);\n"
)]
#[case::object_spread(
    "const __devupA = { s: { c: 1 } }; f({ ...__devupA.s, d: 2 });",
    "const __devupA = { s: { c: 1 } };\nf({\n\tc: 1,\n\td: 2\n});\n"
)]
#[case::object_read_kept(
    "const __devupA = { s: { c: 1 } }; f(__devupA.s);",
    "const __devupA = { s: { c: 1 } };\nf(__devupA.s);\n"
)]
#[case::undefined(
    "const __devupA = { a: undefined }; f(__devupA.a);",
    "const __devupA = { a: undefined };\nf(undefined);\n"
)]
#[case::undefined_shadowed_kept(
    "function g(undefined) { const __devupA = { a: undefined }; return __devupA.a; }",
    "function g(undefined) {\n\tconst __devupA = { a: undefined };\n\treturn __devupA.a;\n}\n"
)]
#[case::authored_kept("const a = { b: 1 }; f(a.b);", "const a = { b: 1 };\nf(a.b);\n")]
#[case::spread_last(
    "const __devupA = { a: 1 }; const __devupB = { b: `x`, ...__devupA }; f(__devupA.a, __devupB.a, __devupB.b);",
    "const __devupB = {
	b: `x`,
	a: 1
};
f(1, 1, `x`);
"
)]
#[case::deleted_computed_kept(
    "const __devupA = { a: 1 }; delete __devupA[\"a\"]; f(__devupA.a);",
    "const __devupA = { a: 1 };
delete __devupA[\"a\"];
f(__devupA.a);
"
)]
#[case::literal_spread(
    "const __devupA = { a: 1 }; f({ ...__devupA.a, b });",
    "const __devupA = { a: 1 };
f({
	...1,
	b
});
"
)]
#[case::let_kept(
    "let __devupA = { b: 1 }; f(__devupA.b);",
    "let __devupA = { b: 1 };\nf(__devupA.b);\n"
)]
#[case::shadowed_spread_kept(
    "const __devupA = { ...p }; const __devupB = { ...__devupA, c: 1 }; f(__devupA.y); function g() { const __devupA = 1; return { ...__devupB.x }; }",
    "const __devupA = { ...p };
const __devupB = {
	...__devupA,
	c: 1
};
f(__devupA.y);
function g() {
	const __devupA = 1;
	return { ...__devupB.x };
}
"
)]
fn known_members(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::spread_once(
    "const __devupA = { a: 1 }; f({ ...__devupA, b: 2 });",
    "f({\n\ta: 1,\n\tb: 2\n});\n"
)]
#[case::copied_known(
    "const __devupA = { ...p }; const __devupB = { ...__devupA, a: __devupA.x }; f({ ...__devupB });",
    "const __devupA = { ...p };\nf({\n\t...__devupA,\n\ta: __devupA.x\n});\n"
)]
#[case::read_twice_kept(
    "const __devupA = { a: 1 }; f({ ...__devupA }, { ...__devupA });",
    "const __devupA = { a: 1 };\nf({ ...__devupA }, { ...__devupA });\n"
)]
#[case::running_kept(
    "const __devupA = { a: g() }; f({ ...__devupA });",
    "const __devupA = { a: g() };\nf({ ...__devupA });\n"
)]
#[case::copying_unknown_kept(
    "const __devupA = { ...p }; f({ ...__devupA });",
    "const __devupA = { ...p };\nf({ ...__devupA });\n"
)]
#[case::spread_in_array("const __devupA = { a: 1 }; f([...__devupA]);", "f([...{ a: 1 }]);\n")]
#[case::two_declarators_kept(
    "const __devupA = { a: 1 }, b = 2; f({ ...__devupA });",
    "const __devupA = { a: 1 }, b = 2;\nf({ ...__devupA });\n"
)]
#[case::read_before_kept(
    "function g() { return { ...__devupA }; } const __devupA = { a: 1 }; g();",
    "function g() {
	return { ...__devupA };
}
const __devupA = { a: 1 };
g();
"
)]
#[case::let_object_kept(
    "let __devupA = { a: 1 }; f({ ...__devupA });",
    "let __devupA = { a: 1 };\nf({ ...__devupA });\n"
)]
fn objects_inline(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::overridden(
    "const { a, ...rest } = { a: 1, ...p, a: c };",
    "const { a, ...rest } = {\n\t...p,\n\ta: c\n};\n"
)]
#[case::running_kept(
    "const { a, ...rest } = { a: g(), a: c };",
    "const { a, ...rest } = {\n\ta: g(),\n\ta: c\n};\n"
)]
#[case::not_named_kept(
    "const { ...rest } = { b: 1, b: c };",
    "const { ...rest } = {\n\tb: 1,\n\tb: c\n};\n"
)]
#[case::computed_pattern_kept(
    "const { [k]: a, ...rest } = { a: 1, a: c };",
    "const { [k]: a, ...rest } = {\n\ta: 1,\n\ta: c\n};\n"
)]
#[case::not_object_kept(
    "const { a } = p; const [b] = [1, 2];",
    "const { a } = p;\nconst [b] = [1, 2];\n"
)]
#[case::getter_kept(
    "const { a } = { a: 1, get a() { return 2; } };",
    "const { a } = {\n\ta: 1,\n\tget a() {\n\t\treturn 2;\n\t}\n};\n"
)]
fn overridden_keys(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}

#[rstest]
#[case::falsy_parts(
    "f([void 0, \"\", a, null, 0, false].filter(Boolean).join(\" \"));",
    "f([a].filter(Boolean).join(\" \"));\n"
)]
#[case::spread_nothing("f({ ...void 0, ...null, a });", "f({ a });\n")]
#[case::text_values("f(`${\"\"}x${\"y\"}`);", "f(`xy`);\n")]
#[case::text_values_escaped_kept("f(`${\"`\"}${\"$\"}`);", "f(`${\"`\"}${\"$\"}`);\n")]
fn falsy_and_text(#[case] source: &str, #[case] expected: &str) {
    assert_eq!(simplified(source), expected, "{source}");
}
