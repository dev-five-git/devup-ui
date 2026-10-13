use super::super::finite_outcomes::{E1, M2, atom, produced};
use super::*;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use css::style_selector::StyleSelector;

#[test]
#[serial]
fn enum_outcomes_when_presets_are_registered_preserve_exact_responsive_selector_results() {
    // Given: actual registered presets and parsed selector/responsive typography.
    let _keys = TypographyKeys::registered();
    let _debug = DebugMode::enabled();
    let allocator = Allocator::default();
    // When: real default extraction and generation yield an Enum lookup and fallback.
    let (emitted, mut finite) = produced(&AstBuilder::new(&allocator), E1);
    // Then: expected-only atoms fix variant, property, value, level, selector and order.
    let mut expected = Vec::new();
    for preset in ["body-1", "heading", "missing"] {
        let declarations = match preset {
            "body-1" | "heading" => vec![ExtractStyleValue::Static(ExtractStaticStyle::new(
                "typography",
                preset,
                1,
                Some(StyleSelector::from("hover")),
            ))],
            "missing" => vec![],
            _ => panic!("listed registry selection"),
        };
        let text = declarations.iter().map(class_of).collect::<String>();
        let setup = format!("const state={{preset:'{preset}'}};");
        assert_eq!(
            json_evaluated(&emitted, &setup),
            serde_json::json!([text, []])
        );
        expected.push((text, declarations));
    }
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    finite.results.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(finite.results, expected);
}

#[test]
#[serial]
fn public_member_outcomes_when_global_reads_differ_preserve_exact_results() {
    // Given: the authored global flag has three independent getter reads.
    let _debug = DebugMode::enabled();
    let allocator = Allocator::default();
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>css({})}}</ClassNames>;",
        &M2[1..M2.len() - 2]
    );
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    // When: the real public local compiler captures and registers its result.
    assert!(visitor.compile_class_names_call(&mut value));
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert!(matches!(&value, Expression::CallExpression(_)));
    let mut finite = visitor
        .style_values
        .at(span)
        .unwrap_or_else(|| panic!("actual registered local origin"))
        .clone();
    // Then: all eight authored triples select exact raw text with three reads.
    let mut expected = Vec::new();
    for mask in 0_u8..8 {
        let (first, second, third) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
        for key in ["a", "b", "missing"] {
            let bg = if first { "white" } else { "black" };
            let color = match (key, second, third) {
                ("a", true, _) => "red",
                ("b", _, false) => "blue",
                _ => "",
            };
            let mut declarations = vec![atom("background", bg)];
            let color_text = if color.is_empty() {
                String::new()
            } else {
                declarations.push(atom("color", color));
                format!("color-0-{color}--255")
            };
            let text = format!("{color_text} background-0-{bg}--255");
            let setup = format!(
                "const key='{key}',flags=[{first},{second},{third}];let index=0;Object.defineProperty(globalThis,'flag',{{get(){{trace.push('flag');return flags[index++];}}}});"
            );
            assert_eq!(
                json_evaluated(&value, &setup),
                serde_json::json!([text, ["flag", "flag", "flag"]])
            );
            let entry = (text, declarations);
            if !expected.contains(&entry) {
                expected.push(entry);
            }
        }
    }
    assert_eq!(expected.len(), 6);
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    finite.results.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(finite.results, expected);
}
