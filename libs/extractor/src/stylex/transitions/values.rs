use std::borrow::Cow;

pub(crate) fn css_value<'v>(property: &str, value: &'v str) -> Cow<'v, str> {
    if matches!(
        property,
        "anchor-name" | "position-anchor" | "position-try-styles" | "position-try-fallbacks"
    ) && value.trim().starts_with("--")
    {
        Cow::Borrowed(value.trim())
    } else {
        css::optimize_value::optimize_value(value)
    }
}
