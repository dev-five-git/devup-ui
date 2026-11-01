use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case::parentheses("(CALLBACK)")]
#[case::as_type("(CALLBACK) as Function")]
#[case::satisfies("(CALLBACK) satisfies Function")]
#[case::non_null("(CALLBACK)!")]
#[case::instantiation("(CALLBACK)<string>")]
#[case::combined("(((CALLBACK)<string>) as Function)!")]
fn self_reference_identity_survives_syntax_wrappers(
    #[case] wrapper: &str,
    #[values(false, true)] shadowed: bool,
) {
    // Given: wrappers surround a generic callback with either self or parameter value reads.
    let parameter = if shadowed { "rules" } else { "p" };
    let callback =
        format!("function rules<rules>({parameter}:rules){{return {{color:rules.name}};}}");
    let source = wrapper.replace("CALLBACK", &callback);
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("invalid wrapper fixture: {error:?}"));
    // When: the same guard analyzes the isolated expression beneath every supported wrapper.
    let error = super::error(&parsed);
    // Then: only references to the function value are refused.
    assert_eq!(error.is_some(), !shadowed, "{source}: {error:?}");
}
