use super::*;
use crate::extract_style::{
    extract_css::ExtractCss, extract_dynamic_style::ExtractDynamicStyle,
    extract_font_face::ExtractFontFace, extract_import::ExtractImport,
    extract_keyframes::ExtractKeyframes, extract_static_style::ExtractStaticStyle,
};
use css::{
    class_map::{get_class_map, reset_class_map},
    debug::set_debug,
    set_prefix,
    style_selector::AtRuleKind,
};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(None, false, None, false)]
#[case(Some("/canonical/producer.css.ts"), false, None, false)]
#[case(Some("/canonical/producer.css.ts"), true, Some("app-"), false)]
#[case(Some("/canonical/producer.css.ts"), false, None, true)]
#[serial]
fn producer_identity_matches_when_original_scope_is_used(
    #[case] scope: Option<&str>,
    #[case] debug: bool,
    #[case] prefix: Option<&str>,
    #[case] basic: bool,
) {
    reset_class_map();
    set_debug(debug);
    set_prefix(prefix.map(str::to_string));
    let style = if basic {
        ExtractStaticStyle::new_basic("color", "red", 0, None)
    } else {
        ExtractStaticStyle::new("color", "red", 0, None)
    };
    let value = ExtractStyleValue::Static(style);
    let original = value.extract(scope);
    let index = ProducerAtoms::from_styles(&FxHashSet::from_iter([value.clone()]), scope);
    assert!(
        matches!(original, Some(StyleProperty::ClassName(ref class)) if index.get(class) == Some([value].as_slice()))
    );
    set_debug(false);
    set_prefix(None);
}

#[test]
#[serial]
fn effects_do_not_allocate_atomic_classes_when_indexed() {
    reset_class_map();
    let global = StyleSelector::Global("body".into(), "/owner.css.ts".into());
    let at = StyleSelector::At {
        kind: AtRuleKind::Media,
        query: "print".into(),
        selector: None,
        outer: vec![],
        file: Some("/owner.css.ts".into()),
    };
    let values = FxHashSet::from_iter([
        ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, Some(global))),
        ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, Some(at))),
        ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 0, "runtime", None)),
        ExtractStyleValue::Typography("body".into()),
        ExtractStyleValue::Keyframes(ExtractKeyframes::default()),
        ExtractStyleValue::Css(ExtractCss {
            css: "body{}".into(),
            file: "/owner.css.ts".into(),
        }),
        ExtractStyleValue::Import(ExtractImport {
            url: "external.css".into(),
            file: "/owner.css.ts".into(),
        }),
        ExtractStyleValue::FontFace(ExtractFontFace {
            file: "/owner.css.ts".into(),
            properties: Default::default(),
        }),
    ]);
    let index = ProducerAtoms::from_styles(&values, Some("/owner.css.ts"));
    assert_eq!(index.0.len(), 0);
    assert_eq!(get_class_map().len(), 0);
}

#[test]
#[serial]
fn local_vars_and_at_rules_remain_typed_when_indexes_merge() {
    reset_class_map();
    let at = StyleSelector::At {
        kind: AtRuleKind::Media,
        query: "print".into(),
        selector: Some("&:hover".into()),
        outer: vec![],
        file: None,
    };
    let values = FxHashSet::from_iter([
        ExtractStyleValue::Static(ExtractStaticStyle::new("margin", "var(--space)", 0, None)),
        ExtractStyleValue::Static(ExtractStaticStyle::new("--custom", "2px", 0, None)),
        ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 2, Some(at))),
    ]);
    let mut index = ProducerAtoms::default();
    index.merge(ProducerAtoms::from_styles(&values, None));
    index.merge(ProducerAtoms::from_styles(&values, None));
    assert_eq!(index.0.values().map(Vec::len).sum::<usize>(), 3);
    for value in values {
        if let Some(StyleProperty::ClassName(class)) = value.extract(None) {
            assert_eq!(index.get(&class), Some([value].as_slice()));
        }
    }
}

#[test]
fn indexed_tokens_are_classes_when_their_spelling_looks_like_a_local_placeholder()
-> Result<(), boa_engine::JsError> {
    let value = ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None));
    let index = ProducerAtoms(FxHashMap::from_iter([("__style_0__".into(), vec![value])]));
    let mut entry = super::super::StyleEntry::default();
    let mut context = boa_engine::Context::default();
    super::super::operands::compose(
        &boa_engine::JsValue::from(boa_engine::js_string!(
            "__style_0__ external __style_1__ tail"
        )),
        &mut entry,
        &index,
        &mut context,
    )?;
    assert!(
        matches!(entry.operands.as_slice(), [super::super::operands::StyleOperand::Classes(first), super::super::operands::StyleOperand::Base(base), super::super::operands::StyleOperand::Classes(last)] if first == "__style_0__ external" && base == "__style_1__" && last == "tail")
    );
    Ok(())
}
