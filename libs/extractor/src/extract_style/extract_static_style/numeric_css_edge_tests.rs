use super::ExtractStaticStyle;
use crate::extract_style::extract_style_value::ExtractStyleValue;

#[rstest::rstest]
#[case(("animation-duration", "3ms"))]
#[case(("animation-delay", "3ms"))]
#[case(("transition-duration", "3ms"))]
#[case(("transition-delay", "3ms"))]
#[case(("line-clamp", "3"))]
#[case(("border-radius", "12px"))]
fn numeric_css_08_vendor_categories_when_prefixed(
    #[case] declaration: (&str, &str),
    #[values("-webkit-", "webkit-", "-moz-", "moz-", "-ms-", "ms-", "-o-", "o-")] prefix: &str,
    #[values(false, true)] basic: bool,
) {
    let (base, expected) = declaration;
    let property = format!("{prefix}{base}");
    let constructor = if basic {
        ExtractStaticStyle::new_basic
    } else {
        ExtractStaticStyle::new
    };
    let style = constructor(&property, "3", 0, None);
    assert_eq!(
        (style.property(), style.value()),
        (property.as_str(), expected)
    );
}

#[test]
#[serial_test::serial]
fn numeric_css_08_inventory_matches_unprefixed_numeric_form()
-> Result<(), Box<dyn std::error::Error>> {
    let inventory = "
        WebkitBoxAlign MozBoxAlign WebkitBoxDirection MozBoxDirection
        WebkitBoxFlex MozBoxFlex WebkitBoxFlexGroup MozBoxFlexGroup
        WebkitBoxLines MozBoxLines WebkitBoxOrdinalGroup MozBoxOrdinalGroup
        WebkitBoxOrient MozBoxOrient WebkitBoxPack MozBoxPack MozOverflowClipBox
        msAccelerator msBlockProgression msContentZoomChaining msContentZoomLimitMax
        msContentZoomLimitMin msContentZoomSnapPoints msContentZoomSnapType msContentZooming
        msFilter msFlowFrom msFlowInto msGridColumns msGridRows msHighContrastAdjust
        msHyphenateLimitChars msHyphenateLimitLines msHyphenateLimitZone msImeAlign
        msOverflowStyle msScrollChaining msScrollLimitXMax msScrollLimitXMin
        msScrollLimitYMax msScrollLimitYMin msScrollRails msScrollSnapPointsX
        msScrollSnapPointsY msScrollSnapType msScrollTranslation msScrollbar3dlightColor
        msScrollbarArrowColor msScrollbarBaseColor msScrollbarDarkshadowColor msScrollbarFaceColor
        msScrollbarHighlightColor msScrollbarShadowColor msScrollbarTrackColor msTextAutospace
        msTouchSelect msUserSelect msWrapFlow msWrapMargin msWrapThrough
        MozAppearance MozBinding MozBorderBottomColors MozBorderLeftColors
        MozBorderRightColors MozBorderTopColors MozContextProperties MozFloatEdge
        MozForceBrokenImageIcon MozOrient MozOutlineRadiusBottomleft MozOutlineRadiusBottomright
        MozOutlineRadiusTopleft MozOutlineRadiusTopright MozStackSizing MozTextBlink
        MozUserFocus MozUserInput MozUserModify MozWindowDragging MozWindowShadow
        WebkitAppearance WebkitBorderAfterColor WebkitBorderAfterStyle WebkitBorderAfterWidth
        WebkitBorderBeforeColor WebkitBorderBeforeStyle WebkitBorderBeforeWidth
        WebkitBorderEndColor WebkitBorderEndStyle WebkitBorderEndWidth
        WebkitBorderStartColor WebkitBorderStartStyle WebkitBorderStartWidth WebkitBoxReflect
        WebkitLineClamp WebkitMaskAttachment WebkitMaskClip WebkitMaskComposite WebkitMaskImage
        WebkitMaskOrigin WebkitMaskPosition WebkitMaskPositionX WebkitMaskPositionY
        WebkitMaskRepeat WebkitMaskRepeatX WebkitMaskRepeatY WebkitMaskSize
        WebkitOverflowScrolling WebkitTapHighlightColor WebkitTextFillColor
        WebkitTextStrokeColor WebkitTextStrokeWidth WebkitTouchCallout WebkitUserModify WebkitUserSelect
        msContentZoomLimit msContentZoomSnap msScrollLimit msScrollSnapX msScrollSnapY
        MozOutlineRadius WebkitBorderAfter WebkitBorderBefore WebkitBorderEnd WebkitBorderStart
        WebkitMask WebkitTextStroke
        msFlex msFlexOrder msFlexPositive msFlexNegative msGridColumnSpan msGridRowSpan MozTabSize
        WebkitAnimationDuration WebkitAnimationDelay WebkitTransitionDuration WebkitTransitionDelay
        MozAnimationDuration MozAnimationDelay MozTransitionDuration MozTransitionDelay
        msAnimationDuration msAnimationDelay msTransitionDuration msTransitionDelay
        OAnimationDuration OAnimationDelay OTransitionDuration OTransitionDelay
    ";
    for key in inventory.split_whitespace() {
        let Some((prefix, suffix)) = [
            ("Webkit", "webkit"),
            ("Moz", "moz"),
            ("ms", "ms"),
            ("O", "o"),
        ]
        .into_iter()
        .find_map(|(prefix, css)| key.strip_prefix(prefix).map(|suffix| (css, suffix))) else {
            panic!("inventory entry needs a vendor prefix: {key}");
        };
        let base = css::utils::to_kebab_case(suffix);
        let property = format!("-{prefix}-{base}");
        let expected = ExtractStaticStyle::new(&base, "3", 0, None);
        let source =
            format!("import {{ Box }} from '@devup-ui/react'; const box = <Box {key}={{3}} />;");
        let output = crate::extract(
            "vendor-inventory.tsx",
            &source,
            crate::ExtractOption::default(),
        )?;
        assert_eq!(output.styles.len(), 1, "{key}");
        let Some(ExtractStyleValue::Static(style)) = output.styles.iter().next() else {
            panic!("expected static vendor declaration: {key}");
        };
        assert_eq!(
            (style.property(), style.value()),
            (property.as_str(), expected.value()),
            "{key}"
        );
    }
    Ok(())
}

#[rstest::rstest]
#[case("-ms-flex")]
#[case("-ms-flex-order")]
#[case("-ms-flex-positive")]
#[case("-ms-flex-negative")]
#[case("-webkit-box-flex")]
#[case("-webkit-box-ordinal-group")]
#[case("-ms-grid-column-span")]
#[case("-ms-grid-row-span")]
#[case("-moz-tab-size")]
#[case("-webkit-line-clamp")]
#[case("-o-opacity")]
#[case("o-opacity")]
fn numeric_css_08_constructors_keep_vendor_numbers(#[case] property: &str) {
    let value = "2";
    let styles = [
        ExtractStaticStyle::new(property, value, 0, None),
        ExtractStaticStyle::new_basic(property, value, 0, None),
    ];
    for style in styles {
        assert_eq!(style.value(), value);
        assert_eq!(style.property(), property);
    }
}

#[rstest::rstest]
#[case("msFlex", "-ms-flex", "2")]
#[case("msFlexOrder", "-ms-flex-order", "2")]
#[case("msFlexPositive", "-ms-flex-positive", "2")]
#[case("msFlexNegative", "-ms-flex-negative", "2")]
#[case("WebkitBoxFlex", "-webkit-box-flex", "2")]
#[case("WebkitBoxOrdinalGroup", "-webkit-box-ordinal-group", "2")]
#[case("msGridColumnSpan", "-ms-grid-column-span", "2")]
#[case("msGridRowSpan", "-ms-grid-row-span", "2")]
#[case("MozTabSize", "-moz-tab-size", "2")]
#[case("WebkitLineClamp", "-webkit-line-clamp", "2")]
#[case("flex", "flex", "2")]
#[case("flexGrow", "flex-grow", "2")]
#[case("flexShrink", "flex-shrink", "2")]
#[case("flexBasis", "flex-basis", "8px")]
#[case("p", "padding", "8px")]
#[case("m", "margin", "8px")]
#[case("WebkitAnimationDuration", "-webkit-animation-duration", "2ms")]
#[serial_test::serial]
fn numeric_css_08_emits_classified_declarations(
    #[case] key: &str,
    #[case] property: &str,
    #[case] value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source =
        format!("import {{ Box }} from '@devup-ui/react'; const box = <Box {key}={{2}} />;");
    let output = crate::extract(
        "numeric-css-08.tsx",
        &source,
        crate::ExtractOption::default(),
    )?;
    assert_eq!(output.styles.len(), 1);
    let Some(ExtractStyleValue::Static(style)) = output.styles.iter().next() else {
        panic!("expected one static declaration");
    };
    assert_eq!((style.property(), style.value()), (property, value));
    Ok(())
}

#[rstest::rstest]
#[case(
    "import { css } from '@emotion/react'; export const box = css({ msFlex: 2, WebkitBoxFlex: 2, msGridColumnSpan: 2, MozTabSize: 2, WebkitLineClamp: 2 });",
    "numbers.ts"
)]
#[case(
    "import { style } from '@vanilla-extract/css'; export const box = style({ msFlex: 2, WebkitBoxFlex: 2, msGridColumnSpan: 2, MozTabSize: 2, WebkitLineClamp: 2 });",
    "numbers.css.ts"
)]
#[serial_test::serial]
fn numeric_css_08_library_callers_emit_bare_vendor_numbers(
    #[case] source: &str,
    #[case] filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let option = crate::ExtractOption {
        import_aliases: std::collections::HashMap::from([
            (
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            ),
            (
                "@vanilla-extract/css".to_string(),
                crate::ImportAlias::NamedToNamed,
            ),
        ]),
        ..crate::ExtractOption::default()
    };
    let output = crate::extract(filename, source, option)?;
    let values: std::collections::BTreeSet<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => (style.property(), style.value()),
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Dynamic(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::Keyframes(_) => panic!("expected static vendor declarations"),
        })
        .collect();
    assert_eq!(
        values,
        std::collections::BTreeSet::from([
            ("-ms-flex", "2"),
            ("-webkit-box-flex", "2"),
            ("-ms-grid-column-span", "2"),
            ("-moz-tab-size", "2"),
            ("-webkit-line-clamp", "2"),
        ])
    );
    Ok(())
}
