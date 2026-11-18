//! What a `css()` class gives Emotion's `<Global>`.

use css::style_selector::StyleSelector;

use crate::extract_style::{
    extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
};

/// What a `css()` class given to `<Global>` requires
pub(crate) const GLOBAL_CLASS: &str = "a `css()` class given to `<Global>` must hold only rules under selectors, as `css({ body: { margin: 0 } })`";

/// The rules a `css()` class holds under selectors, as global rules of those
/// selectors, as Emotion's `<Global>` writes them; `None` when the class holds
/// a declaration of its own or a rule no plain selector names
pub(crate) fn global_class_styles(
    styles: &[ExtractStyleValue],
    file: &str,
) -> Option<Vec<ExtractStyleValue>> {
    styles
        .iter()
        .map(|style| {
            let ExtractStyleValue::Static(style) = style else {
                return None;
            };
            let (selector, at_rule) = match style.selector()? {
                StyleSelector::Selector(selector) => (selector, None),
                StyleSelector::At {
                    kind,
                    query,
                    selector: Some(selector),
                    outer,
                    ..
                } if outer.is_empty() => (selector, Some((*kind, query))),
                _ => return None,
            };
            let global = selector
                .strip_prefix("& ")
                .filter(|global| !global.contains('&'))?;
            let global = StyleSelector::Global(global.to_string(), file.to_string());
            let selector = match at_rule {
                Some((kind, query)) => StyleSelector::nest_at_rule(Some(&global), kind, query)?,
                None => global,
            };
            Some(ExtractStyleValue::Static(ExtractStaticStyle::new(
                style.property(),
                style.value(),
                style.level(),
                Some(selector),
            )))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_class_holding_more_than_rules_is_no_global() {
        assert!(
            global_class_styles(&[ExtractStyleValue::Typography("heading".into())], "a.tsx")
                .is_none()
        );
    }
}
