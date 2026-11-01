use super::*;
use crate::extract_style::{
    extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
};

#[test]
fn residual_union_when_new_keys_and_external_classes_arrive_keeps_their_slots() {
    // Given: a repeated color key, distinct keys, and independent unkeyed class expressions.
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let atom = |property: &str, value: &str| {
        ExtractStyleProp::Static(ExtractStyleValue::Static(ExtractStaticStyle::new(
            property, value, 0, None,
        )))
    };
    let external = || ExtractStyleProp::Expression {
        expression: oxc_parser::Parser::new(&arena, "external", oxc_span::SourceType::mjs())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}")),
        styles: vec![],
    };
    let color = atom("color", "red");
    let opacity = atom("opacity", "0.5");
    let padding = atom("padding", "4px");
    let inherited = shape(
        &ast,
        &[color.clone_in(&arena), opacity.clone_in(&arena), external()],
    );
    let own = shape(
        &ast,
        &[color.clone_in(&arena), padding.clone_in(&arena), external()],
    );
    // When: the residual carrier combines inherited and newly declared slot shapes.
    let joined = inherited.union(&own);
    // Then: only keyed duplicates coalesce; distinct keys and both external slots remain ordered.
    assert!(
        joined.0
            == vec![
                inherited.0[0].clone(),
                inherited.0[1].clone(),
                None,
                own.0[1].clone(),
                None
            ]
    );
    let merged = inherited.merge(&own, "frame", "own");
    let script = format!(
        "const frame=[['old-color','old-opacity','old-external']];const own=['new-color','new-padding','new-external'];JSON.stringify({merged});"
    );
    let actual = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected JSON observation"))
        .to_std_string_escaped();
    assert_eq!(
        actual,
        r#"["new-color","old-opacity","old-external","new-padding","new-external"]"#
    );
}
