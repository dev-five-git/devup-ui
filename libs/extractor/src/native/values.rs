//! Values as the source writes them, and as React Native reads them

use std::borrow::Cow;

use crate::extract_style::constant::is_maintain_value_property;
use crate::utils::convert_value;

use super::errors::Problem;

/// The base `rem` and `em` are measured in, in density-independent pixels
pub(super) const ROOT_FONT_SIZE: f64 = 16.0;

/// A value the source writes as a literal
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Literal {
    Number(f64),
    Text(String),
}

/// A value of a React Native style
#[derive(Debug, Clone, PartialEq)]
pub(super) enum NativeValue {
    Number(f64),
    Text(String),
    /// Texts as an array
    List(Vec<String>),
}

impl NativeValue {
    /// The value as JavaScript writes it
    pub(super) fn code(&self) -> String {
        match self {
            Self::Number(number) => number_code(*number),
            Self::Text(text) => serde_json::Value::String(text.clone()).to_string(),
            Self::List(texts) => serde_json::Value::from(texts.clone()).to_string(),
        }
    }
}

/// `number` as JavaScript writes it
pub(super) fn number_code(number: f64) -> String {
    format!("{number}")
}

/// The finite number `text` writes, if it is one
pub(super) fn number(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

/// The CSS value the web build gives `literal` for the CSS `property`: a
/// number is four pixels a unit, but for the properties taking a plain number
/// The properties of an animation or a transition a number of milliseconds or
/// a count is given to, kept as it is
pub(super) const TIMED: [&str; 5] = [
    "animation-duration",
    "animation-delay",
    "animation-iteration-count",
    "transition-duration",
    "transition-delay",
];

pub(super) fn css_text<'a>(property: &str, literal: &'a Literal) -> Cow<'a, str> {
    let text = match literal {
        Literal::Number(number) => Cow::Owned(number_code(*number)),
        Literal::Text(text) => Cow::Borrowed(text.as_str()),
    };
    if is_maintain_value_property(property) || TIMED.contains(&property) {
        return text;
    }
    match convert_value(&text) {
        Cow::Owned(converted) => Cow::Owned(converted),
        Cow::Borrowed(_) => text,
    }
}

/// A length: pixels and `rem`/`em` become numbers, a percentage and `auto`
/// stay strings
pub(super) fn length(text: &str) -> Result<NativeValue, Problem> {
    let text = text.trim();
    if text == "auto" || text.strip_suffix('%').and_then(number).is_some() {
        return Ok(NativeValue::Text(text.to_string()));
    }
    scaled_length(text)
        .or_else(|| number(text))
        .map(NativeValue::Number)
        .ok_or_else(|| unsupported_length(text))
}

/// `text` in pixels, from pixels or a `rem`/`em` length
pub(super) fn scaled_length(text: &str) -> Option<f64> {
    [("px", 1.0), ("rem", ROOT_FONT_SIZE), ("em", ROOT_FONT_SIZE)]
        .into_iter()
        .find_map(|(unit, scale)| {
            text.strip_suffix(unit)
                .and_then(number)
                .map(|number| number * scale)
        })
}

/// What a viewport length measures a hundredth of
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Viewport {
    Width,
    Height,
    /// The smaller of the width and the height
    Min,
    /// The larger of the width and the height
    Max,
}

/// The viewport units, by what they measure. A window has no browser
/// interface that comes and goes, so the small, large and dynamic units
/// measure the window as the plain ones do; the inline and block axes are
/// the width and the height
const VIEWPORT_UNITS: [(&str, Viewport); 24] = [
    ("vw", Viewport::Width),
    ("svw", Viewport::Width),
    ("lvw", Viewport::Width),
    ("dvw", Viewport::Width),
    ("vi", Viewport::Width),
    ("vh", Viewport::Height),
    ("svh", Viewport::Height),
    ("lvh", Viewport::Height),
    ("dvh", Viewport::Height),
    ("vb", Viewport::Height),
    ("vmin", Viewport::Min),
    ("svmin", Viewport::Min),
    ("lvmin", Viewport::Min),
    ("dvmin", Viewport::Min),
    ("vmax", Viewport::Max),
    ("svmax", Viewport::Max),
    ("lvmax", Viewport::Max),
    ("dvmax", Viewport::Max),
    ("svi", Viewport::Width),
    ("lvi", Viewport::Width),
    ("dvi", Viewport::Width),
    ("svb", Viewport::Height),
    ("lvb", Viewport::Height),
    ("dvb", Viewport::Height),
];

/// The number of hundredths `text` measures and of what, for a viewport
/// length such as `50vw`
pub(super) fn viewport_length(text: &str) -> Option<(f64, Viewport)> {
    let text = text.trim();
    VIEWPORT_UNITS.iter().find_map(|(unit, viewport)| {
        text.strip_suffix(unit)
            .and_then(number)
            .map(|amount| (amount, *viewport))
    })
}

fn unsupported_length(text: &str) -> Problem {
    if viewport_length(text).is_some() {
        return Problem::new(
            format!(
                "`{text}` is a viewport length, which the build computes only where a style prop writes it"
            ),
            "write the viewport length in the style prop, or use a number of pixels or a percentage",
        );
    }
    Problem::new(
        format!("`{text}` is not one length React Native reads"),
        "use one number of pixels, a `rem` or `em` length, a percentage or `auto`",
    )
}

/// A plain number, such as an opacity or a flex factor
pub(super) fn plain_number(text: &str) -> Result<NativeValue, Problem> {
    number(text)
        .map(NativeValue::Number)
        .ok_or_else(|| Problem::new(format!("`{text}` is not a number"), "use a number"))
}

/// A number when it is one, else the text, as `aspectRatio` takes `16 / 9`
pub(super) fn number_or_text(text: &str) -> NativeValue {
    number(text).map_or_else(
        || NativeValue::Text(text.trim().to_string()),
        NativeValue::Number,
    )
}

/// A font weight, which React Native reads as a string
pub(super) fn font_weight(text: &str) -> NativeValue {
    NativeValue::Text(text.trim().to_string())
}

/// A line height, which React Native reads in pixels, never relative to the
/// font size
pub(super) fn line_height(text: &str) -> Result<NativeValue, Problem> {
    let text = text.trim();
    scaled_length(text)
        .map(NativeValue::Number)
        .ok_or_else(|| {
            Problem::new(
                format!(
                    "`{text}` is relative to the font size on the web, but React Native reads a line height in pixels"
                ),
                "write the line height in pixels, such as `24px`",
            )
        })
}

/// One of the `keywords` React Native reads for the property
pub(super) fn keyword(text: &str, keywords: &[&str]) -> Result<NativeValue, Problem> {
    let text = text.trim();
    if keywords.contains(&text) {
        return Ok(NativeValue::Text(text.to_string()));
    }
    Err(Problem::new(
        format!("React Native does not read `{text}` here"),
        format!("use one of {}", quoted_list(keywords)),
    ))
}

/// `values` quoted and joined, the last by `or`
pub(super) fn quoted_list(values: &[&str]) -> String {
    let quoted: Vec<String> = values.iter().map(|value| format!("`{value}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => quoted.concat(),
    }
}
