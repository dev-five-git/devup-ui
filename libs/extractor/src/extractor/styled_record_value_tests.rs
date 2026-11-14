use super::static_lookup_value;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("null", true, true)]
#[case("({color:null, nested:{color:'red'}})", true, true)]
#[case("({color:read()})", true, false)]
#[case("({nested:{color:'red'}})", false, false)]
#[case("({color:'red'})", false, true)]
#[case("read()", false, false)]
fn finite_lookup_when_values_are_literal_data_is_safe_to_select(
    #[case] source: &str,
    #[case] rules: bool,
    #[case] expected: bool,
) {
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));

    let actual = static_lookup_value(&expression, rules);

    assert_eq!(actual, expected, "{source}");
}
