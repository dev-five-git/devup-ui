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
use super::theme::NativeTheme;
use super::values::{
    Literal, NativeValue, css_text, font_weight, keyword, length, line_height, number, number_code,
    number_or_text, plain_number, scaled_length,
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
    /// Milliseconds or a count as a number, or a time or `infinite` as text,
    /// which a value read at runtime is given as it is
    Timed,
    /// What `keyframes()` gives, which no literal writes
    Keyframes,
    /// The CSS properties a transition animates, each under its React Native
    /// key
    TransitionProperty,
    /// Transitions, each led by the CSS property it animates
    Transition,
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
    "animation-name" => ("animationName", Kind::Keyframes),
    "animation-duration" => ("animationDuration", Kind::Timed),
    "animation-delay" => ("animationDelay", Kind::Timed),
    "animation-timing-function" => ("animationTimingFunction", Kind::Text),
    "animation-iteration-count" => ("animationIterationCount", Kind::Timed),
    "animation-direction" => ("animationDirection", Kind::Keyword(&["normal", "reverse", "alternate", "alternate-reverse"])),
    "animation-fill-mode" => ("animationFillMode", Kind::Keyword(&["none", "forwards", "backwards", "both"])),
    "animation-play-state" => ("animationPlayState", Kind::Keyword(&["running", "paused"])),
    "transition" => ("transition", Kind::Transition),
    "transition-property" => ("transitionProperty", Kind::TransitionProperty),
    "transition-duration" => ("transitionDuration", Kind::Timed),
    "transition-delay" => ("transitionDelay", Kind::Timed),
    "transition-timing-function" => ("transitionTimingFunction", Kind::Text),
    "transition-behavior" => ("transitionBehavior", Kind::Keyword(&["normal", "allow-discrete"])),
};

/// A style React Native reads: its key and value
pub(super) type Declaration = (String, NativeValue);

/// The React Native key and kind of the CSS `property`
fn native_property(property: &str) -> Result<(&'static str, Kind), Problem> {
    NATIVE_PROPERTIES.get(property).copied().ok_or_else(|| {
        if property == "animation" {
            return Problem::new(
                "react-native-reanimated reads the longhands of `animation`, and the keyframes as an object `keyframes()` gives",
                "write `animationName={keyframes({ ... })}` with `animationDuration` and the other longhands",
            );
        }
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
        Kind::Ratio | Kind::Timed => Ok(number_or_text(text)),
        Kind::FontWeight => Ok(font_weight(text)),
        Kind::LineHeight => line_height(text),
        Kind::Text => Ok(NativeValue::Text(text.trim().to_string())),
        Kind::Keyword(keywords) => keyword(text, keywords),
        Kind::Keyframes => Err(Problem::new(
            "`animationName` takes the keyframes `keyframes()` gives on React Native, not a name",
            "pass what `keyframes({ ... })` gives",
        )),
        Kind::TransitionProperty => Ok(transition_properties(text)),
        Kind::Transition => Ok(NativeValue::Text(
            text.split(',')
                .map(transition_part)
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

/// The React Native key of the CSS `property` a transition names, or the name
/// as it is for one React Native has no style for, such as `all`
fn transition_key(property: &str) -> &str {
    native_property(property).map_or(property, |(key, _)| key)
}

/// The properties of `transition-property`: one name, or a list of them
fn transition_properties(text: &str) -> NativeValue {
    let keys: Vec<String> = text
        .split(',')
        .map(|property| transition_key(property.trim()).to_string())
        .collect();
    match <[String; 1]>::try_from(keys) {
        Ok([key]) => NativeValue::Text(key),
        Err(keys) => NativeValue::List(keys),
    }
}

/// One transition of `transition`, with the property leading it under its
/// React Native key
fn transition_part(part: &str) -> String {
    let part = part.trim();
    let (property, rest) = part.split_once(char::is_whitespace).unwrap_or((part, ""));
    let key = transition_key(property);
    if rest.is_empty() {
        key.to_string()
    } else {
        format!("{key} {}", rest.trim_start())
    }
}

/// What the web cannot pass on to React Native in any value
fn check_text(text: &str) -> Result<(), Problem> {
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

/// Where the `$token`s of a value resolve: one theme of the theme a file is
/// compiled with
pub(super) struct Tokens<'t> {
    pub theme: &'t NativeTheme,
    /// The index of the theme
    pub index: usize,
    /// The breakpoint
    pub level: usize,
}

impl Tokens<'_> {
    /// `text` with its tokens resolved, or `None` when the theme lacks one
    fn resolve<'v>(&self, text: &'v str) -> Result<Option<Cow<'v, str>>, Problem> {
        self.theme.resolve(text, (self.index, self.level))
    }
}

/// The styles the style prop `prop`, with the value `literal`, declares in
/// the theme of `tokens`, or `None` when that theme lacks a token it names
pub(super) fn declarations(
    prop: &str,
    literal: &Literal,
    tokens: &Tokens<'_>,
) -> Result<Option<Vec<Declaration>>, Problem> {
    if prop == "typography" {
        return typography(&literal_text(literal), tokens);
    }
    if css::is_enum_property(prop) {
        return enum_declarations(prop, literal).map(Some);
    }
    let mut declarations = Vec::new();
    for property in css::disassemble_property(prop) {
        let text = css_text(&property, literal);
        let Some(text) = tokens.resolve(&text)? else {
            return Ok(None);
        };
        check_text(&text)?;
        declarations.extend(property_declarations(&property, &text)?);
    }
    Ok(Some(declarations))
}

/// How many breakpoints the styles of the style prop `prop` with the value
/// `literal` tell apart through the responsive tokens or typography it names
pub(super) fn literal_levels(prop: &str, literal: &Literal, theme: &NativeTheme) -> usize {
    match literal {
        Literal::Text(text) if prop == "typography" => theme.typography_levels(text),
        Literal::Text(text) => theme.token_levels(text),
        Literal::Number(_) => 1,
    }
}

/// `literal` as text
fn literal_text(literal: &Literal) -> Cow<'_, str> {
    match literal {
        Literal::Text(text) => Cow::Borrowed(text.as_str()),
        Literal::Number(number) => Cow::Owned(number_code(*number)),
    }
}

/// The styles of the typography `name` of the theme: its font size in
/// pixels, and a line height or letter spacing relative to the font size
/// multiplied by it, as React Native reads both in pixels
fn typography(name: &str, tokens: &Tokens<'_>) -> Result<Option<Vec<Declaration>>, Problem> {
    let mut properties = Vec::new();
    for (property, value) in tokens.theme.typography(name, tokens.level)? {
        let Some(value) = tokens.resolve(&value)?.map(Cow::into_owned) else {
            return Ok(None);
        };
        check_text(&value)?;
        properties.push((property, value));
    }
    let font_size = properties
        .iter()
        .find(|(property, _)| property == "font-size")
        .and_then(|(_, value)| scaled_length(value.trim()).or_else(|| number(value)));
    let mut declarations = Vec::new();
    for (property, value) in &properties {
        match property.as_str() {
            "line-height" | "letter-spacing" => {
                declarations.extend(relative_to_font(property, value, font_size)?);
            }
            property => declarations.extend(property_declarations(property, value)?),
        }
    }
    Ok(Some(declarations))
}

/// A typography's line height or letter spacing in pixels: a number, a
/// percentage or an `em` length is relative to the font size, and `normal`
/// leaves React Native's default
fn relative_to_font(
    property: &str,
    text: &str,
    font_size: Option<f64>,
) -> Result<Option<Declaration>, Problem> {
    let text = text.trim();
    let key = if property == "line-height" {
        "lineHeight"
    } else {
        "letterSpacing"
    };
    if text == "normal" {
        return Ok(None);
    }
    let relative = text
        .strip_suffix('%')
        .and_then(number)
        .map(|percent| percent / 100.0)
        .or_else(|| text.strip_suffix("em").and_then(number))
        .or_else(|| (property == "line-height").then(|| number(text)).flatten());
    let pixels = match (relative, font_size) {
        // Rounded to a thousandth of a pixel, so `1.3` of `24px` is `31.2`
        (Some(relative), Some(font_size)) => (relative * font_size * 1000.0).round() / 1000.0,
        (Some(_), None) => {
            return Err(Problem::new(
                format!(
                    "`{text}` is relative to the font size, and the typography has no font size in pixels"
                ),
                "give the typography a `fontSize` in pixels, or the value in pixels",
            ));
        }
        (None, _) => return length(text).map(|value| Some((key.to_string(), value))),
    };
    Ok(Some((key.to_string(), NativeValue::Number(pixels))))
}

/// The styles of a Devup UI enum prop, such as `positioning="top-left"`
fn enum_declarations(prop: &str, literal: &Literal) -> Result<Vec<Declaration>, Problem> {
    let value = literal_text(literal);
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
    if prop == "typography" {
        return Err(Problem::new(
            "the build reads the styles of a typography by its name, which a value read at runtime does not give",
            "write the name, or a condition between names such as `typography={big ? 'h1' : 'body'}`",
        ));
    }
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

/// The React Native keys a viewport length of the style prop `prop` sets,
/// when each takes a length
pub(super) fn length_keys(prop: &str) -> Option<Vec<&'static str>> {
    css::disassemble_property(prop)
        .map(|property| match property.as_ref() {
            "margin" => Some("margin"),
            "padding" => Some("padding"),
            property => native_property(property)
                .ok()
                .filter(|(_, kind)| matches!(kind, Kind::Length | Kind::LineHeight))
                .map(|(key, _)| key),
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
        Kind::FontWeight
        | Kind::Text
        | Kind::Keyword(_)
        | Kind::Timed
        | Kind::Keyframes
        | Kind::TransitionProperty
        | Kind::Transition => None,
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
