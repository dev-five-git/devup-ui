use super::*;
use rstest::rstest;

fn applied(policy: StyleResolution, key: &str, value: Option<&str>) -> Vec<ExpandedKey> {
    expansion(policy, key)
        .map(|expansion| expansion.apply(value))
        .and_then(Result::ok)
        .unwrap_or_default()
}

fn set(key: &'static str, value: &str) -> ExpandedKey {
    (key, Some(value.to_string()))
}

#[rstest]
#[case(StyleResolution::PropertySpecificity, "padding")]
#[case(StyleResolution::ApplicationOrder, "color")]
#[case(StyleResolution::ApplicationOrder, "padding-top")]
#[case(StyleResolution::LegacyExpandShorthands, "float")]
#[case(StyleResolution::LegacyExpandShorthands, "clear")]
#[case(StyleResolution::LegacyExpandShorthands, "flex")]
fn keys_without_an_expansion_stay(#[case] policy: StyleResolution, #[case] key: &str) {
    assert!(expansion(policy, key).is_none());
}

#[test]
fn application_order_keeps_the_value_and_resets_what_it_covers() {
    assert_eq!(
        applied(
            StyleResolution::ApplicationOrder,
            "padding",
            Some("1px 2px")
        ),
        vec![
            set("padding", "1px 2px"),
            ("paddingInline", None),
            ("paddingStart", None),
            ("paddingLeft", None),
            ("paddingEnd", None),
            ("paddingRight", None),
            ("paddingBlock", None),
            ("paddingTop", None),
            ("paddingBottom", None),
        ]
    );
    assert_eq!(
        applied(
            StyleResolution::ApplicationOrder,
            "marginStart",
            Some("4px")
        ),
        vec![
            set("marginInlineStart", "4px"),
            ("marginLeft", None),
            ("marginRight", None)
        ]
    );
    assert_eq!(
        applied(StyleResolution::ApplicationOrder, "blockSize", None),
        vec![("height", None)]
    );
}

#[test]
fn application_order_rejects_all() {
    assert_eq!(
        expansion(StyleResolution::ApplicationOrder, "all").map(|e| e.apply(Some("unset"))),
        Some(Err("all is not supported".to_string()))
    );
}

#[rstest]
#[case("margin", "1px", ["1px", "1px", "1px", "1px"])]
#[case("margin", "1px 2px", ["1px", "2px", "1px", "2px"])]
#[case("margin", "1px 2px 3px", ["1px", "2px", "3px", "2px"])]
#[case("margin", "1px 2px 3px 4px 5px", ["1px", "2px", "3px", "4px"])]
#[case("padding", "1px 2px !important", ["1px !important", "2px !important", "1px !important", "2px !important"])]
#[case("borderColor", "rgb(0, 0, 0) red", ["rgb(0, 0, 0)", "red", "rgb(0, 0, 0)", "red"])]
fn legacy_spreads_a_box_over_four_sides(
    #[case] key: &str,
    #[case] value: &str,
    #[case] sides: [&str; 4],
) {
    let values: Vec<Option<String>> =
        applied(StyleResolution::LegacyExpandShorthands, key, Some(value))
            .into_iter()
            .map(|(_, value)| value)
            .collect();
    assert_eq!(values, sides.map(|side| Some(side.to_string())).to_vec());
}

#[test]
fn legacy_keeps_logical_sides_and_resets_physical_ones() {
    assert_eq!(
        applied(
            StyleResolution::LegacyExpandShorthands,
            "padding",
            Some("1px 2px 3px 4px")
        ),
        vec![
            set("paddingTop", "1px"),
            set("paddingInlineEnd", "2px"),
            set("paddingBottom", "3px"),
            set("paddingInlineStart", "4px"),
        ]
    );
    assert_eq!(
        applied(
            StyleResolution::LegacyExpandShorthands,
            "marginInline",
            Some("1px 2px")
        ),
        vec![
            set("marginInlineStart", "1px"),
            set("marginInlineEnd", "2px"),
            ("marginLeft", None),
            ("marginRight", None),
        ]
    );
    assert_eq!(
        applied(StyleResolution::LegacyExpandShorthands, "gap", Some("1px")),
        vec![set("rowGap", "1px"), set("columnGap", "1px")]
    );
    assert_eq!(
        applied(
            StyleResolution::LegacyExpandShorthands,
            "border",
            Some("1px solid red")
        ),
        vec![
            set("borderTop", "1px solid red"),
            set("borderInlineEnd", "1px solid red"),
            set("borderBottom", "1px solid red"),
            set("borderInlineStart", "1px solid red"),
        ]
    );
    assert_eq!(
        applied(StyleResolution::LegacyExpandShorthands, "overflow", None),
        vec![("overflowX", None), ("overflowY", None)]
    );
}

#[rstest]
#[case("auto 10px", ["auto 10px", "auto 10px"])]
#[case("10px auto 20px", ["10px", "auto 20px"])]
#[case("10px 20px", ["10px", "20px"])]
#[case("auto", ["auto", "auto"])]
fn legacy_contain_intrinsic_size_joins_auto(#[case] value: &str, #[case] sizes: [&str; 2]) {
    assert_eq!(
        applied(
            StyleResolution::LegacyExpandShorthands,
            "containIntrinsicSize",
            Some(value)
        ),
        vec![
            set("containIntrinsicWidth", sizes[0]),
            set("containIntrinsicHeight", sizes[1]),
        ]
    );
}

#[rstest]
#[case("square inside", [Some("square"), Some("inside"), None])]
#[case("inherit", [Some("inherit"), Some("inherit"), Some("inherit")])]
#[case("none url(a.png)", [Some("none"), None, Some("url(a.png)")])]
#[case("disc none", [Some("disc"), None, Some("none")])]
#[case("'-' outside", [Some("'-'"), Some("outside"), None])]
#[case("\"x\"", [Some("\"x\""), None, None])]
#[case("", [None, None, Some("")])]
fn legacy_list_style_sorts_its_parts(#[case] value: &str, #[case] parts: [Option<&str>; 3]) {
    assert_eq!(
        applied(
            StyleResolution::LegacyExpandShorthands,
            "listStyle",
            Some(value)
        ),
        vec![
            ("listStyleType", parts[0].map(str::to_string)),
            ("listStylePosition", parts[1].map(str::to_string)),
            ("listStyleImage", parts[2].map(str::to_string)),
        ]
    );
}

#[rstest]
#[case("square inherit")]
#[case("var(--x) inside")]
#[case("inside outside")]
#[case("square disc")]
#[case("url(a.png) url(b.png)")]
fn legacy_list_style_rejects_what_stylex_rejects(#[case] value: &str) {
    assert_eq!(
        expansion(StyleResolution::LegacyExpandShorthands, "listStyle")
            .map(|e| e.apply(Some(value))),
        Some(Err(format!("invalid \"listStyle\" value of {value:?}")))
    );
}

#[rstest]
#[case("1px  2px", &["1px", "2px"])]
#[case(" a,b/c ", &["a", "b", "c"])]
#[case("calc(1px + 2px) 3px", &["calc(1px + 2px)", "3px"])]
#[case("\"a b\" 'c d'", &["\"a b\"", "'c d'"])]
#[case("f(g(1 2)) x)", &["f(g(1 2))", "x)"])]
#[case("!important", &["!important"])]
#[case("1px !IMPORTANT", &["1px !important"])]
#[case("", &[""])]
#[case("   ", &[""])]
fn values_split_as_stylex_splits_them(#[case] value: &str, #[case] parts: &[&str]) {
    assert_eq!(split_value(value), parts);
}

#[test]
fn tables_name_camel_case_keys() {
    for table in [
        &tables::APPLICATION_ORDER,
        &tables::LEGACY_EXPAND_SHORTHANDS,
    ] {
        for (key, expansion) in table.entries() {
            for name in std::iter::once(key)
                .chain(expansion.targets)
                .chain(expansion.resets)
            {
                assert!(!name.contains('-'), "{name}");
            }
        }
    }
}
