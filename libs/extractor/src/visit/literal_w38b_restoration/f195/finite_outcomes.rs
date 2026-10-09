use super::*;
use crate::ExtractStyleProp::{Conditional, Enum, MemberExpression, StaticArray};
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::gen_class_name::gen_class_names;
use crate::{ExtractStyleValue::Static, finite_styles::FiniteStyles as Finite};
const M1: &str = "({color:({a:'red',b:'blue'})[key]});";
pub(super) const M2: &str =
    "({bg:flag?'white':'black',color:({a:flag?'red':null,b:!flag?'blue':null})[key]});";
pub(super) const E1: &str = "({_hover:{typography:[null,state.preset]}});";
pub(super) fn produced<'a>(ast: &AstBuilder<'a>, source: &'a str) -> (Expression<'a>, Finite) {
    let mut value = parsed(ast.allocator(), source);
    let handling = LiteralHandling::ExpandResponsiveThemeToken;
    let mut result = extract_style_from_expression(ast, None, &mut value, 0, &None, handling);
    match (source, result.styles.as_slice()) {
        (M1, [MemberExpression { .. }]) | (M2, [MemberExpression { .. }, Conditional { .. }]) => {}
        (E1, [StaticArray(props)]) if matches!(props.as_slice(), [Enum { .. }]) => {}
        _ => panic!("actual source style shape: {:#?}", result.styles),
    }
    let emitted = gen_class_names(ast, &mut result.styles, None, None)
        .unwrap_or_else(|| panic!("actual generated lookup"));
    let finite = Finite::emitted(ast, &result.styles, &emitted)
        .unwrap_or_else(|| panic!("actual yielded finite outcomes"));
    (emitted, finite)
}

pub(super) fn atom(property: &str, value: &str) -> ExtractStyleValue {
    Static(ExtractStaticStyle::new(property, value, 0, None))
}

#[rstest]
#[case(M1)]
#[case(M2)]
#[serial]
fn member_outcomes_when_source_is_correlated_preserve_exact_results(#[case] source: &str) {
    // Given: parsed Member sources with a single shared plain flag.
    let _debug = DebugMode::enabled();
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    // When: real default extraction/emission supplies finite results.
    let (emitted, mut finite) = produced(&ast, source);
    // Then: independent static atoms fix the entire result/declaration correlation.
    let mut expected = Vec::new();
    for key in ["a", "b", "missing"] {
        for flag in [true, false] {
            let color = match (key, flag, source == M1) {
                ("a", true, _) | ("a", false, true) => "red",
                ("b", false, _) | ("b", true, true) => "blue",
                _ => "",
            };
            let bg = if flag { "white" } else { "black" };
            let mut declarations = Vec::new();
            if source == M2 {
                declarations.push(atom("background", bg));
            }
            let color_text = if color.is_empty() {
                String::new()
            } else {
                declarations.push(atom("color", color));
                format!("color-0-{color}--255")
            };
            let background = format!("background-0-{bg}--255");
            let text = match source {
                M1 => color_text,
                M2 => format!("{background} {color_text}"),
                _ => panic!("listed Member source"),
            };
            let setup = format!("const key='{key}',flag={flag};");
            assert_eq!(
                json_evaluated(&emitted, &setup),
                serde_json::json!([text, []])
            );
            let entry = (text, declarations);
            if !expected.contains(&entry) {
                expected.push(entry);
            }
        }
    }
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    finite.results.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(finite.results, expected);
}
