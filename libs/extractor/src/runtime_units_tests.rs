use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_codegen::Codegen;
use oxc_span::SPAN;
use rstest::rstest;

use super::*;
use crate::ImportAlias;

fn print(value: &Expression<'_>) -> String {
    let mut codegen = Codegen::new();
    codegen.print_expression(value);
    codegen.into_source_text()
}

/// What the helper returns for `value`, run as the compiled file runs it
fn run(value: &str, scale: &str) -> String {
    let script = format!(
        "{}JSON.stringify(__devupScale({value}{scale}) ?? null)",
        module_script()
    );
    boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .ok()
        .and_then(|value| value.as_string().map(|text| text.to_std_string_escaped()))
        .unwrap_or_else(|| panic!("{script}"))
}

#[rstest]
#[case("2", "", "\"8px\"")]
#[case("'2'", "", "\"8px\"")]
#[case("-1.5", "", "\"-6px\"")]
#[case("'.5'", "", "\"2px\"")]
#[case("'1e1'", "", "\"40px\"")]
#[case("0", "", "\"0px\"")]
#[case("3", ", 1, 'ms'", "\"3ms\"")]
#[case("'2px'", "", "\"2px\"")]
#[case("'auto'", "", "\"auto\"")]
#[case("'$space'", "", "\"$space\"")]
#[case("''", "", "\"\"")]
#[case("null", "", "null")]
#[case("undefined", "", "null")]
#[case("false", "", "false")]
#[case("[2]", "", "[2]")]
#[case("{ toString() { throw 1; } }", "", "{}")]
fn helper_converts_only_what_a_literal_converts(
    #[case] value: &str,
    #[case] scale: &str,
    #[case] expected: &str,
) {
    assert_eq!(run(value, scale), expected);
}

#[rstest]
#[case("padding", Some(Unit::Length))]
#[case("margin-left", Some(Unit::Length))]
#[case("width", Some(Unit::Length))]
#[case("font-size", Some(Unit::Length))]
#[case("background", Some(Unit::Length))]
#[case("-webkit-box-flex", None)]
#[case("transition-duration", Some(Unit::Time))]
#[case("-webkit-animation-delay", Some(Unit::Time))]
#[case("opacity", None)]
#[case("z-index", None)]
#[case("line-height", None)]
#[case("flex", None)]
#[case("color", None)]
#[case("border-color", None)]
#[case("-webkit-text-fill-color", None)]
#[case("--custom", None)]
fn unit_mirrors_how_a_literal_is_converted(#[case] property: &str, #[case] expected: Option<Unit>) {
    assert_eq!(unit(property), expected);
}

#[rstest]
#[case("m", true)]
#[case("a ? b : c", true)]
#[case("`${m}`", true)]
#[case("`2`", true)]
#[case("`2.`", true)]
#[case("``", true)]
#[case("`", true)]
#[case("`${m}px`", false)]
#[case("`${m}%`", false)]
fn may_be_number_when_the_value_ends_as_a_number_can(#[case] code: &str, #[case] expected: bool) {
    assert_eq!(may_be_number(code), expected);
}

#[test]
fn guard_scopes_the_helper_to_one_file() {
    assert!(!active());
    assert_eq!(runtime_unit("padding"), None);
    {
        let _outer = Guard::new(Some("outer".into()));
        assert!(active());
        assert_eq!(runtime_unit("padding"), Some(Unit::Length));
        assert_eq!(runtime_unit("opacity"), None);
        {
            let _inner = Guard::new(None);
            assert!(!active());
        }
        assert_eq!(helper().as_deref(), Some("outer"));
    }
    assert!(!active());
}

#[rstest]
#[case(Some("h"), Some(Unit::Length), "h(m)")]
#[case(Some("h"), Some(Unit::Time), "h(m, 1, \"ms\")")]
#[case(Some("h"), None, "m")]
#[case(None, Some(Unit::Length), "m")]
fn wrap_calls_the_helper_only_for_a_unit(
    #[case] name: Option<&str>,
    #[case] unit: Option<Unit>,
    #[case] expected: &str,
) {
    let _guard = Guard::new(name.map(str::to_owned));
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let value = wrap(
        &builder,
        unit,
        Expression::new_identifier(SPAN, "m", &builder),
    );
    assert_eq!(print(&value), expected);
    assert_eq!(print(converted(&value)), "m");
}

#[rstest]
#[case(Some("h"), "other(m)")]
#[case(Some("h"), "h.call(m)")]
#[case(None, "h(m)")]
fn converted_sees_only_through_the_helper(#[case] name: Option<&str>, #[case] code: &str) {
    let _guard = Guard::new(name.map(str::to_owned));
    let allocator = Allocator::default();
    let value = oxc_parser::Parser::new(&allocator, code, oxc_span::SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|errors| panic!("{errors:?}"));
    assert_eq!(print(converted(&value)), print(&value));
}

#[test]
fn marked_never_tells_styles_apart() {
    use std::hash::{BuildHasher, RandomState};
    let state = RandomState::new();
    assert_eq!(Marked(true), Marked(false));
    assert_eq!(Marked(true).cmp(&Marked(false)), std::cmp::Ordering::Equal);
    assert_eq!(
        Marked(true).partial_cmp(&Marked(false)),
        Some(std::cmp::Ordering::Equal)
    );
    assert_eq!(state.hash_one(Marked(true)), state.hash_one(Marked(false)));
}

#[rstest]
#[case("import {Box} from '@devup-ui/react'", "a.tsx", &[], false, true)]
#[case("import {Box} from '@devup-ui/react'", "a.tsx", &[], true, false)]
#[case("import {style} from '@vanilla-extract/css'", "a.css.ts", &[], false, false)]
#[case(
    "import styled from '@emotion/styled'",
    "a.tsx",
    &["@emotion/styled"],
    false,
    false
)]
#[case("import {Box} from '@devup-ui/react'", "a.tsx", &["@emotion/styled"], false, true)]
fn applies_only_where_numbers_follow_devup_units(
    #[case] source: &str,
    #[case] filename: &str,
    #[case] packages: &[&str],
    #[case] library_rules: bool,
    #[case] expected: bool,
) {
    let aliases: HashMap<String, ImportAlias> = packages
        .iter()
        .map(|package| {
            (
                (*package).to_owned(),
                ImportAlias::DefaultToNamed("styled".into()),
            )
        })
        .collect();
    assert_eq!(applies(source, filename, &aliases, library_rules), expected);
}

#[rstest]
#[case(
    WEB_EXPORT,
    "__devupScale",
    false,
    "import { __devupScale } from \"p/scale\";\n"
)]
#[case(
    WEB_EXPORT,
    "h",
    false,
    "import { __devupScale as h } from \"p/scale\";\n"
)]
#[case(
    NATIVE_EXPORT,
    "__u",
    false,
    "import { __devupNativeScale as __u } from \"p/scale\";\n"
)]
#[case(
    WEB_EXPORT,
    "h",
    true,
    "const h = require(\"p/scale\").__devupScale;\n"
)]
fn import_code_binds_the_local_name(
    #[case] export: &str,
    #[case] local: &str,
    #[case] commonjs: bool,
    #[case] expected: &str,
) {
    assert_eq!(import_code("p", export, local, commonjs), expected);
}

/// `code` with the web conversion imported as `h` when it reads it
fn imported(code: &str, source_type: oxc_span::SourceType) -> String {
    let allocator = Allocator::default();
    let mut program = oxc_parser::Parser::new(&allocator, code, source_type)
        .parse()
        .program;
    import(&allocator, &mut program, "p", "h");
    Codegen::new().build(&program).code
}

#[rstest]
#[case(
    "import a from \"a\";\na(h(m));\n",
    "import a from \"a\";\nimport { __devupScale as h } from \"p/scale\";\na(h(m));\n"
)]
#[case("a(m);\n", "a(m);\n")]
#[case("h(m);\n", "import { __devupScale as h } from \"p/scale\";\nh(m);\n")]
fn import_follows_the_leading_imports_only_when_read(#[case] code: &str, #[case] expected: &str) {
    assert_eq!(imported(code, oxc_span::SourceType::mjs()), expected);
}

#[test]
fn import_requires_in_commonjs() {
    assert_eq!(
        imported("h(m);\n", oxc_span::SourceType::cjs()),
        "const h = require(\"p/scale\").__devupScale;\nh(m);\n"
    );
}
