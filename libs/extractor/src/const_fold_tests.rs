use super::*;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

fn folded(code: &str) -> String {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut expression = Parser::new(&allocator, code, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    fold(&ast, &mut expression);
    crate::utils::readable_code(&expression)
}

#[rstest]
// A test whose answer is a literal picks its side
#[case("!0 ? 1 : 2", "1")]
#[case("!'a' ? 1 : 2", "2")]
#[case("!x", "!x")]
#[case("null ? 1 : 2", "2")]
#[case("'' ? 1 : 2", "2")]
#[case("`` ? 1 : 2", "2")]
#[case("`a` ? 1 : 2", "1")]
#[case("`${x}` ? 1 : 2", "`${x}` ? 1 : 2")]
#[case("NaN ? 1 : 2", "NaN ? 1 : 2")]
#[case("-1 ? 1 : 2", "1")]
#[case("+0 ? 1 : 2", "2")]
#[case("-x ? 1 : 2", "-x ? 1 : 2")]
#[case("typeof 1 ? 1 : 2", "typeof 1 ? 1 : 2")]
#[case("void 0 ? 1 : 2", "2")]
#[case("void x ? 1 : 2", "void x ? 1 : 2")]
#[case("x ? 1 : 2", "x ? 1 : 2")]
#[case("1 && 2", "2")]
#[case("0 && 2", "0")]
#[case("1 || 2", "1")]
#[case("0 || 2", "2")]
#[case("null ?? 2", "2")]
#[case("void 0 ?? 2", "2")]
#[case("0 ?? 2", "0")]
#[case("x ?? 2", "x ?? 2")]
#[case("x || 2", "x || 2")]
// Only literals before the last expression are left out
#[case("(0, 'a', 2)", "2")]
#[case("(x, 2)", "x, 2")]
// Arithmetic written as a plain number
#[case("1 + 2", "3")]
#[case("3 - 1", "2")]
#[case("2 * 3", "6")]
#[case("1 - 2", "1 - 2")]
#[case("1e308 * 10", "1e308 * 10")]
#[case("4 / 2", "4 / 2")]
#[case("x + 1", "x + 1")]
fn literal_tests_and_arithmetic_are_written_as_their_value(
    #[case] code: &str,
    #[case] expected: &str,
) {
    assert_eq!(folded(code), expected);
}

#[rstest]
#[case("[1, 2][1]", "2")]
#[case("[1, 2]['1']", "2")]
#[case("[1, 2][`1`]", "2")]
#[case("[1, 2][+1]", "2")]
#[case("[1, 2][-0]", "1")]
#[case("[1, 2][0 + 1]", "2")]
#[case("[1, 2][(0, 1)]", "2")]
#[case("[1, 2][true ? 0 : 1]", "1")]
#[case("[1, , 2][1]", "void 0")]
#[case("[1, 2][10]", "void 0")]
#[case("[1, 2]['10']", "void 0")]
#[case("[1, 2]['01']", "void 0")]
#[case("[1, 2][-1]", "void 0")]
#[case("[[1, 2], [3]][1][5]", "void 0")]
#[case("[1, 2]['length']", "[1, 2][\"length\"]")]
#[case("[1, 2][1.5]", "[1, 2][1.5]")]
#[case("[1, 2][`${x}`]", "[1, 2][`${x}`]")]
#[case("[1, 2][-x]", "[1, 2][-x]")]
#[case("[1, 2][x]", "[1, 2][x]")]
#[case("[1, 2]?.[1]", "[1, 2]?.[1]")]
#[case("[1, ...x][0]", "[1, ...x][0]")]
// The value read is evaluated as written; the others are left out only as literals
#[case("[1, x][1]", "x")]
#[case("[x, 2][1]", "[x, 2][1]")]
#[case("'ab'[0]", "\"ab\"[0]")]
#[case("({ a: 1, b: 2 }).b", "2")]
#[case("({ a: 1, b: 2 })['b']", "2")]
#[case("({ a: 1, a: 2 }).a", "2")]
#[case("({ ...{ a: 1 }, a: 2 }).a", "2")]
#[case("({ a: 1, ...{ a: 2 } }).a", "2")]
#[case("({ a: { b: 1 } }).a.c", "void 0")]
#[case("({ a: 1 }).c", "void 0")]
#[case("({ a: 1 }).toString", "{ a: 1 }.toString")]
#[case("({ toString: 1 }).toString", "1")]
#[case("({ a: 1, ...x }).a", "{\n\ta: 1,\n\t...x\n}.a")]
#[case("({ a: 1, b() {} }).a", "{\n\ta: 1,\n\tb() {}\n}.a")]
#[case("({ a: 1 })?.a", "{ a: 1 }?.a")]
#[case("x.a", "x.a")]
fn literal_keys_into_literal_tables_are_written_as_what_they_read(
    #[case] code: &str,
    #[case] expected: &str,
) {
    let expected = crate::utils::readable_code(
        &Parser::new(&Allocator::default(), expected, SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}")),
    );
    assert_eq!(folded(code), expected, "{code}");
}

#[rstest]
#[case("[1, 2, 3][c ? 0 : 1]", "c ? 1 : 2")]
#[case("[1, 2, 3][c ? 5 : 1]", "c ? void 0 : 2")]
#[case("({ a: 1, b: 2, d: 3 })[c ? 'a' : 'b']", "c ? 1 : 2")]
// A table whose reading evaluates more than literals runs before the test
#[case("[x, 2, 3][c ? 1 : 2]", "[ x, 2, 3 ][c ? 1 : 2]")]
#[case("[1, 2][c ? 0 : k]", "[1, 2][c ? 0 : k]")]
#[case("[1, 2][c ? k : 0]", "[1, 2][c ? k : 0]")]
#[case("[1, 2][c ? 'length' : 0]", "[1, 2][c ? \"length\" : 0]")]
fn conditional_keys_into_literal_tables_choose_between_what_they_read(
    #[case] code: &str,
    #[case] expected: &str,
) {
    assert_eq!(folded(code), expected, "{code}");
}

#[test]
fn style_orders_are_left_as_written() {
    assert_eq!(
        folded("({ styleOrder: !1, p: !1 })"),
        "({ styleOrder: !1, p: false })"
    );
}
