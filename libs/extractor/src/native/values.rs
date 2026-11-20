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
}

impl NativeValue {
    /// The value as JavaScript writes it
    pub(super) fn code(&self) -> String {
        match self {
            Self::Number(number) => number_code(*number),
            Self::Text(text) => serde_json::Value::String(text.clone()).to_string(),
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
pub(super) fn css_text<'a>(property: &str, literal: &'a Literal) -> Cow<'a, str> {
    let text = match literal {
        Literal::Number(number) => Cow::Owned(number_code(*number)),
        Literal::Text(text) => Cow::Borrowed(text.as_str()),
    };
    if is_maintain_value_property(property) {
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

fn unsupported_length(text: &str) -> Problem {
    if ["vw", "vh", "vmin", "vmax", "dvh", "svh", "lvh"]
        .iter()
        .any(|unit| text.ends_with(unit))
    {
        return Problem::new(
            format!("`{text}` needs the window size, which React Native knows only at runtime"),
            "use a number of pixels or a percentage",
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
