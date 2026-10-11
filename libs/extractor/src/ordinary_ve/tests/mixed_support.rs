use crate::{ExtractOutput, ExtractStyleValue, extract_with_modules};

pub(super) fn extract(
    extension: &str,
    source: &str,
) -> Result<ExtractOutput, Box<dyn std::error::Error>> {
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    extract_with_modules(
        &format!("/mixed.{extension}"),
        source,
        super::option(),
        false,
        &|_, _| None,
    )
}

pub(super) fn has_static(output: &ExtractOutput, property: &str, value: &str) -> bool {
    output.styles.iter().any(|style| {
        matches!(style, ExtractStyleValue::Static(style)
            if style.property == property && style.value == value)
    })
}

pub(super) fn css_payload(output: &ExtractOutput) -> Vec<ExtractStyleValue> {
    let mut styles: Vec<_> = output.styles.iter().cloned().collect();
    for style in &mut styles {
        match style {
            ExtractStyleValue::Css(css) => css.file.clear(),
            ExtractStyleValue::FontFace(font) => font.file.clear(),
            ExtractStyleValue::Import(import) => import.file.clear(),
            ExtractStyleValue::Static(style) => match &mut style.selector {
                Some(css::style_selector::StyleSelector::Global(_, file)) => file.clear(),
                Some(css::style_selector::StyleSelector::At { file, .. }) => *file = None,
                Some(css::style_selector::StyleSelector::Selector(_)) | None => {}
            },
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Dynamic(_)
            | ExtractStyleValue::Keyframes(_) => {}
        }
    }
    styles.sort();
    styles
}

pub(super) fn assert_consumed(output: &ExtractOutput) {
    assert!(
        !output.code.contains("@vanilla-extract/css"),
        "{}",
        output.code
    );
    assert!(!output.code.contains("__style_"), "{}", output.code);
    for api in [
        "style",
        "globalStyle",
        "styleVariants",
        "keyframes",
        "fontFace",
        "globalFontFace",
        "createVar",
        "fallbackVar",
        "createContainer",
        "layer",
        "globalLayer",
        "createThemeContract",
        "createGlobalThemeContract",
        "assignVars",
        "createTheme",
        "createGlobalTheme",
    ] {
        assert!(!output.code.contains(&format!("{api}(")), "{}", output.code);
    }
}
