//! CSS properties as React Native styles: which React Native reads, under
//! which key, and how their values convert.
//!
//! The list follows the style types of React Native with the New Architecture
//! (`ViewStyle`, `TextStyle` and `ImageStyle`). A CSS property it does not
//! name is a build error.

use std::borrow::Cow;

use phf::phf_map;

use crate::extract_style::constant::is_maintain_value_property;

use super::errors::Problem;
use super::values::{
    Literal, NativeValue, css_text, font_weight, keyword, length, line_height, number_code,
    number_or_text, plain_number,
};

/// How React Native reads the value of a style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// Pixels as a number, or a percentage or `auto`
    Length,
    /// A plain number
    Number,
    /// A number, or a ratio as text
    Ratio,
    /// A weight, as a string
    FontWeight,
    /// Pixels only
    LineHeight,
    /// Text kept as it is: colors, shadows, transforms, filters, families
    Text,
    /// One of a set of keywords
    Keyword(&'static [&'static str]),
}

const ALIGN: &[&str] = &["flex-start", "flex-end", "center", "stretch", "baseline"];
const ALIGN_SELF: &[&str] = &[
    "auto",
    "flex-start",
    "flex-end",
    "center",
    "stretch",
    "baseline",
];
const DISTRIBUTE: &[&str] = &[
    "flex-start",
    "flex-end",
    "center",
    "stretch",
    "space-between",
    "space-around",
    "space-evenly",
];
const JUSTIFY: &[&str] = &[
    "flex-start",
    "flex-end",
    "center",
    "space-between",
    "space-around",
    "space-evenly",
];
const LINE_STYLE: &[&str] = &["solid", "dotted", "dashed"];
const TEXT_DECORATION_LINE: &[&str] = &[
    "none",
    "underline",
    "line-through",
    "underline line-through",
];

/// The React Native key and value kind of each CSS property React Native reads
static NATIVE_PROPERTIES: phf::Map<&'static str, (&'static str, Kind)> = phf_map! {
    "width" => ("width", Kind::Length),
    "height" => ("height", Kind::Length),
    "min-width" => ("minWidth", Kind::Length),
    "min-height" => ("minHeight", Kind::Length),
    "max-width" => ("maxWidth", Kind::Length),
    "max-height" => ("maxHeight", Kind::Length),
    "margin" => ("margin", Kind::Length),
    "margin-top" => ("marginTop", Kind::Length),
    "margin-right" => ("marginRight", Kind::Length),
    "margin-bottom" => ("marginBottom", Kind::Length),
    "margin-left" => ("marginLeft", Kind::Length),
    "margin-block" => ("marginBlock", Kind::Length),
    "margin-block-start" => ("marginBlockStart", Kind::Length),
    "margin-block-end" => ("marginBlockEnd", Kind::Length),
    "margin-inline" => ("marginInline", Kind::Length),
    "margin-inline-start" => ("marginInlineStart", Kind::Length),
    "margin-inline-end" => ("marginInlineEnd", Kind::Length),
    "padding" => ("padding", Kind::Length),
    "padding-top" => ("paddingTop", Kind::Length),
    "padding-right" => ("paddingRight", Kind::Length),
    "padding-bottom" => ("paddingBottom", Kind::Length),
    "padding-left" => ("paddingLeft", Kind::Length),
    "padding-block" => ("paddingBlock", Kind::Length),
    "padding-block-start" => ("paddingBlockStart", Kind::Length),
    "padding-block-end" => ("paddingBlockEnd", Kind::Length),
    "padding-inline" => ("paddingInline", Kind::Length),
    "padding-inline-start" => ("paddingInlineStart", Kind::Length),
    "padding-inline-end" => ("paddingInlineEnd", Kind::Length),
    "top" => ("top", Kind::Length),
    "right" => ("right", Kind::Length),
    "bottom" => ("bottom", Kind::Length),
    "left" => ("left", Kind::Length),
    "inset" => ("inset", Kind::Length),
    "inset-block" => ("insetBlock", Kind::Length),
    "inset-block-start" => ("insetBlockStart", Kind::Length),
    "inset-block-end" => ("insetBlockEnd", Kind::Length),
    "inset-inline" => ("insetInline", Kind::Length),
    "inset-inline-start" => ("insetInlineStart", Kind::Length),
    "inset-inline-end" => ("insetInlineEnd", Kind::Length),
    "gap" => ("gap", Kind::Length),
    "row-gap" => ("rowGap", Kind::Length),
    "column-gap" => ("columnGap", Kind::Length),
    "flex-basis" => ("flexBasis", Kind::Length),
    "border-width" => ("borderWidth", Kind::Length),
    "border-top-width" => ("borderTopWidth", Kind::Length),
    "border-right-width" => ("borderRightWidth", Kind::Length),
    "border-bottom-width" => ("borderBottomWidth", Kind::Length),
    "border-left-width" => ("borderLeftWidth", Kind::Length),
    "border-radius" => ("borderRadius", Kind::Length),
    "border-top-left-radius" => ("borderTopLeftRadius", Kind::Length),
    "border-top-right-radius" => ("borderTopRightRadius", Kind::Length),
    "border-bottom-left-radius" => ("borderBottomLeftRadius", Kind::Length),
    "border-bottom-right-radius" => ("borderBottomRightRadius", Kind::Length),
    "border-start-start-radius" => ("borderStartStartRadius", Kind::Length),
    "border-start-end-radius" => ("borderStartEndRadius", Kind::Length),
    "border-end-start-radius" => ("borderEndStartRadius", Kind::Length),
    "border-end-end-radius" => ("borderEndEndRadius", Kind::Length),
    "outline-width" => ("outlineWidth", Kind::Length),
    "outline-offset" => ("outlineOffset", Kind::Length),
    "font-size" => ("fontSize", Kind::Length),
    "letter-spacing" => ("letterSpacing", Kind::Length),
    "flex" => ("flex", Kind::Number),
    "flex-grow" => ("flexGrow", Kind::Number),
    "flex-shrink" => ("flexShrink", Kind::Number),
    "opacity" => ("opacity", Kind::Number),
    "z-index" => ("zIndex", Kind::Number),
    "aspect-ratio" => ("aspectRatio", Kind::Ratio),
    "font-weight" => ("fontWeight", Kind::FontWeight),
    "line-height" => ("lineHeight", Kind::LineHeight),
    "background-color" => ("backgroundColor", Kind::Text),
    "background-image" => ("backgroundImage", Kind::Text),
    "color" => ("color", Kind::Text),
    "border-color" => ("borderColor", Kind::Text),
    "border-top-color" => ("borderTopColor", Kind::Text),
    "border-right-color" => ("borderRightColor", Kind::Text),
    "border-bottom-color" => ("borderBottomColor", Kind::Text),
    "border-left-color" => ("borderLeftColor", Kind::Text),
    "border-block-color" => ("borderBlockColor", Kind::Text),
    "border-block-start-color" => ("borderBlockStartColor", Kind::Text),
    "border-block-end-color" => ("borderBlockEndColor", Kind::Text),
    "outline-color" => ("outlineColor", Kind::Text),
    "text-decoration-color" => ("textDecorationColor", Kind::Text),
    "box-shadow" => ("boxShadow", Kind::Text),
    "filter" => ("filter", Kind::Text),
    "transform" => ("transform", Kind::Text),
    "transform-origin" => ("transformOrigin", Kind::Text),
    "font-family" => ("fontFamily", Kind::Text),
    "font-variant" => ("fontVariant", Kind::Text),
    "display" => ("display", Kind::Keyword(&["none", "flex", "contents"])),
    "position" => ("position", Kind::Keyword(&["absolute", "relative", "static"])),
    "overflow" => ("overflow", Kind::Keyword(&["visible", "hidden", "scroll"])),
    "flex-direction" => ("flexDirection", Kind::Keyword(&["row", "column", "row-reverse", "column-reverse"])),
    "flex-wrap" => ("flexWrap", Kind::Keyword(&["wrap", "nowrap", "wrap-reverse"])),
    "justify-content" => ("justifyContent", Kind::Keyword(JUSTIFY)),
    "align-items" => ("alignItems", Kind::Keyword(ALIGN)),
    "align-self" => ("alignSelf", Kind::Keyword(ALIGN_SELF)),
    "align-content" => ("alignContent", Kind::Keyword(DISTRIBUTE)),
    "direction" => ("direction", Kind::Keyword(&["inherit", "ltr", "rtl"])),
    "box-sizing" => ("boxSizing", Kind::Keyword(&["border-box", "content-box"])),
    "backface-visibility" => ("backfaceVisibility", Kind::Keyword(&["visible", "hidden"])),
    "border-style" => ("borderStyle", Kind::Keyword(LINE_STYLE)),
    "outline-style" => ("outlineStyle", Kind::Keyword(LINE_STYLE)),
    "pointer-events" => ("pointerEvents", Kind::Keyword(&["auto", "none", "box-none", "box-only"])),
    "cursor" => ("cursor", Kind::Keyword(&["auto", "pointer"])),
    "isolation" => ("isolation", Kind::Keyword(&["auto", "isolate"])),
    "mix-blend-mode" => ("mixBlendMode", Kind::Text),
    "font-style" => ("fontStyle", Kind::Keyword(&["normal", "italic"])),
    "text-align" => ("textAlign", Kind::Keyword(&["auto", "left", "right", "center", "justify"])),
    "text-decoration" => ("textDecorationLine", Kind::Keyword(TEXT_DECORATION_LINE)),
    "text-decoration-line" => ("textDecorationLine", Kind::Keyword(TEXT_DECORATION_LINE)),
    "text-decoration-style" => ("textDecorationStyle", Kind::Keyword(&["solid", "double", "dotted", "dashed"])),
    "text-transform" => ("textTransform", Kind::Keyword(&["none", "uppercase", "lowercase", "capitalize"])),
    "user-select" => ("userSelect", Kind::Keyword(&["auto", "none", "text", "contain", "all"])),
    "vertical-align" => ("verticalAlign", Kind::Keyword(&["auto", "top", "bottom", "middle"])),
    "object-fit" => ("objectFit", Kind::Keyword(&["cover", "contain", "fill", "scale-down", "none"])),
};

/// A style React Native reads: its key and value
pub(super) type Declaration = (String, NativeValue);

/// The React Native key and kind of the CSS `property`
fn native_property(property: &str) -> Result<(&'static str, Kind), Problem> {
    NATIVE_PROPERTIES.get(property).copied().ok_or_else(|| {
        Problem::new(
            format!("React Native has no `{property}` style"),
            "remove it, or write a style React Native reads",
        )
    })
}

/// The value `text` as React Native reads a value of `kind`
fn native_value(kind: Kind, text: &str) -> Result<NativeValue, Problem> {
    match kind {
        Kind::Length => length(text),
        Kind::Number => plain_number(text),
        Kind::Ratio => Ok(number_or_text(text)),
        Kind::FontWeight => Ok(font_weight(text)),
        Kind::LineHeight => line_height(text),
        Kind::Text => Ok(NativeValue::Text(text.trim().to_string())),
        Kind::Keyword(keywords) => keyword(text, keywords),
    }
}

/// What the web cannot pass on to React Native in any value
fn check_text(text: &str) -> Result<(), Problem> {
    if text.contains('$') {
        return Err(super::errors::theme_token(text));
    }
    if text.contains("!important") {
        return Err(Problem::new(
            "React Native styles have no `!important`",
            "remove `!important`; a later style already wins",
        ));
    }
    if text.contains("var(") || text.contains("calc(") || text.contains("env(") {
        return Err(Problem::new(
            format!("React Native cannot compute `{text}`"),
            "write the value itself",
        ));
    }
    Ok(())
}

/// The styles the style prop `prop`, with the value `literal`, declares
pub(super) fn declarations(prop: &str, literal: &Literal) -> Result<Vec<Declaration>, Problem> {
    if css::is_enum_property(prop) {
        return enum_declarations(prop, literal);
    }
    let mut declarations = Vec::new();
    for property in css::disassemble_property(prop) {
        let text = css_text(&property, literal);
        check_text(&text)?;
        declarations.extend(property_declarations(&property, &text)?);
    }
    Ok(declarations)
}

/// The styles of a Devup UI enum prop, such as `positioning="top-left"`
fn enum_declarations(prop: &str, literal: &Literal) -> Result<Vec<Declaration>, Problem> {
    let value = match literal {
        Literal::Text(text) => Cow::Borrowed(text.as_str()),
        Literal::Number(number) => Cow::Owned(number_code(*number)),
    };
    let map = css::get_enum_property_value(prop, &value).ok_or_else(|| {
        Problem::new(
            format!("`{prop}` has no `{value}`"),
            "use one of the values the prop takes",
        )
    })?;
    let mut declarations = Vec::new();
    for (property, value) in map.entries() {
        declarations.extend(property_declarations(property, value)?);
    }
    declarations.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(declarations)
}

/// The styles the CSS `property: text` declares
fn property_declarations(property: &str, text: &str) -> Result<Vec<Declaration>, Problem> {
    match property {
        "background" => background(text).map(|declaration| vec![declaration]),
        "margin" | "padding" => box_sides(property, text),
        "gap" => gap(text),
        "border" => Ok(border(text)),
        _ => {
            let (key, kind) = native_property(property)?;
            Ok(vec![(key.to_string(), native_value(kind, text)?)])
        }
    }
}

/// `background`: a gradient is a background image, else a color
fn background(text: &str) -> Result<Declaration, Problem> {
    let text = text.trim();
    if text.contains("url(") {
        return Err(Problem::new(
            "React Native draws no image in a background",
            "render an `Image` instead",
        ));
    }
    let key = if text.contains("gradient(") {
        "backgroundImage"
    } else {
        "backgroundColor"
    };
    Ok((key.to_string(), NativeValue::Text(text.to_string())))
}

/// `margin` and `padding` with one to four lengths, side by side
fn box_sides(property: &str, text: &str) -> Result<Vec<Declaration>, Problem> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let sides: &[&str] = match parts.len() {
        1 => &[""],
        2 => &["Vertical", "Horizontal"],
        3 => &["Top", "Horizontal", "Bottom"],
        4 => &["Top", "Right", "Bottom", "Left"],
        _ => {
            return Err(Problem::new(
                format!("`{text}` is not one to four lengths"),
                "give one to four lengths",
            ));
        }
    };
    parts
        .iter()
        .zip(sides)
        .map(|(part, side)| Ok((format!("{property}{side}"), length(part)?)))
        .collect()
}

/// `gap` with a row and a column gap
fn gap(text: &str) -> Result<Vec<Declaration>, Problem> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    match parts.as_slice() {
        [row, column] => Ok(vec![
            ("rowGap".to_string(), length(row)?),
            ("columnGap".to_string(), length(column)?),
        ]),
        _ => Ok(vec![("gap".to_string(), length(text)?)]),
    }
}

/// `border` as its width, style and color
fn border(text: &str) -> Vec<Declaration> {
    let text = text.trim();
    if text == "none" || text == "0" {
        return vec![("borderWidth".to_string(), NativeValue::Number(0.0))];
    }
    let mut declarations = Vec::new();
    let mut color = Vec::new();
    for part in text.split_whitespace() {
        if LINE_STYLE.contains(&part) {
            declarations.push((
                "borderStyle".to_string(),
                NativeValue::Text(part.to_string()),
            ));
        } else if let Ok(width) = length(part) {
            declarations.push(("borderWidth".to_string(), width));
        } else {
            color.push(part);
        }
    }
    if !color.is_empty() {
        declarations.push((
            "borderColor".to_string(),
            NativeValue::Text(color.join(" ")),
        ));
    }
    declarations
}

/// The React Native keys a style prop declares with a value the element
/// reads at runtime, each with the scale a number of it is multiplied by
/// when the value is converted as the build converts a literal, or `None`
/// when React Native receives it as it is
pub(super) fn dynamic_keys(prop: &str) -> Result<Vec<(&'static str, Option<u32>)>, Problem> {
    if css::is_enum_property(prop) {
        return Err(runtime_value_problem(prop));
    }
    css::disassemble_property(prop)
        .map(|property| match property.as_ref() {
            "background" => Ok(("backgroundColor", None)),
            "margin" => Ok(("margin", runtime_scale("margin", Kind::Length))),
            "padding" => Ok(("padding", runtime_scale("padding", Kind::Length))),
            "border" => Err(runtime_value_problem(prop)),
            property => {
                native_property(property).map(|(key, kind)| (key, runtime_scale(property, kind)))
            }
        })
        .collect()
}

/// The scale a number read at runtime for the CSS `property` of `kind` is
/// multiplied by, as the web build scales a literal: four pixels a unit for a
/// length, but for the properties taking a plain number
fn runtime_scale(property: &str, kind: Kind) -> Option<u32> {
    match kind {
        Kind::Length if !is_maintain_value_property(property) => Some(4),
        Kind::Length | Kind::Number | Kind::Ratio | Kind::LineHeight => Some(1),
        Kind::FontWeight | Kind::Text | Kind::Keyword(_) => None,
    }
}

fn runtime_value_problem(prop: &str) -> Problem {
    Problem::new(
        format!(
            "the build cannot split a value of `{prop}` it reads at runtime into React Native styles"
        ),
        "write the value as a literal, or pass the React Native styles in `style`",
    )
}
