//! Utilities of Tailwind CSS v4 that are a fixed set of declarations or one
//! value on a few properties: text decoration and clamping, border widths and
//! styles, spacing on logical sides, table and transform keywords

use std::borrow::Cow;

use css::style_selector::AtRuleKind;

use crate::tailwind::{
    Declaration, Nested, TRANSFORM, ValueType, arbitrary_or_variable, bracketed, decl,
    decode_arbitrary_value, is_positive_integer, spacing_scale, split_top_level, value_type,
};
use crate::tailwind_children::spacing;
use crate::tailwind_color::color_value;
use crate::tailwind_theme::themed;

/// The utilities that are one fixed list of declarations
fn fixed(name: &str) -> Option<&'static [(&'static str, &'static str)]> {
    Some(match name {
        "field-sizing-content" => &[("field-sizing", "content")],
        "field-sizing-fixed" => &[("field-sizing", "fixed")],
        "table-auto" => &[("table-layout", "auto")],
        "table-fixed" => &[("table-layout", "fixed")],
        "caption-top" => &[("caption-side", "top")],
        "caption-bottom" => &[("caption-side", "bottom")],
        "transform-flat" => &[("transform-style", "flat")],
        "transform-3d" => &[("transform-style", "preserve-3d")],
        "transform-none" => &[("transform", "none")],
        "backface-visible" => &[("backface-visibility", "visible")],
        "backface-hidden" => &[("backface-visibility", "hidden")],
        "forced-color-adjust-none" => &[("forced-color-adjust", "none")],
        "forced-color-adjust-auto" => &[("forced-color-adjust", "auto")],
        "antialiased" => &[
            ("-webkit-font-smoothing", "antialiased"),
            ("-moz-osx-font-smoothing", "grayscale"),
        ],
        "subpixel-antialiased" => &[
            ("-webkit-font-smoothing", "auto"),
            ("-moz-osx-font-smoothing", "auto"),
        ],
        "scheme-normal" => &[("color-scheme", "normal")],
        "scheme-dark" => &[("color-scheme", "dark")],
        "scheme-light" => &[("color-scheme", "light")],
        "scheme-light-dark" => &[("color-scheme", "light dark")],
        "scheme-only-dark" => &[("color-scheme", "only dark")],
        "scheme-only-light" => &[("color-scheme", "only light")],
        "wrap-anywhere" => &[("overflow-wrap", "anywhere")],
        "wrap-break-word" => &[("overflow-wrap", "break-word")],
        "wrap-normal" => &[("overflow-wrap", "normal")],
        "decoration-solid" => &[("text-decoration-style", "solid")],
        "decoration-double" => &[("text-decoration-style", "double")],
        "decoration-dotted" => &[("text-decoration-style", "dotted")],
        "decoration-dashed" => &[("text-decoration-style", "dashed")],
        "decoration-wavy" => &[("text-decoration-style", "wavy")],
        "decoration-auto" => &[("text-decoration-thickness", "auto")],
        "decoration-from-font" => &[("text-decoration-thickness", "from-font")],
        "underline-offset-auto" => &[("text-underline-offset", "auto")],
        "list-image-none" => &[("list-style-image", "none")],
        "normal-nums" => &[("font-variant-numeric", "normal")],
        "contain-none" => &[("contain", "none")],
        "contain-content" => &[("contain", "content")],
        "contain-strict" => &[("contain", "strict")],
        "scale-3d" => &[(
            "scale",
            "var(--tw-scale-x) var(--tw-scale-y) var(--tw-scale-z)",
        )],
        "translate-3d" => &[(
            "translate",
            "var(--tw-translate-x) var(--tw-translate-y) var(--tw-translate-z)",
        )],
        "transform-cpu" => &[("transform", TRANSFORM)],
        "transform-gpu" => &[(
            "transform",
            "translateZ(0) var(--tw-rotate-x,) var(--tw-rotate-y,) var(--tw-rotate-z,) var(--tw-skew-x,) var(--tw-skew-y,)",
        )],
        "mask-none" => &[("mask-image", "none")],
        "mask-add" => &[("mask-composite", "add")],
        "mask-subtract" => &[("mask-composite", "subtract")],
        "mask-intersect" => &[("mask-composite", "intersect")],
        "mask-exclude" => &[("mask-composite", "exclude")],
        "mask-alpha" => &[("mask-mode", "alpha")],
        "mask-luminance" => &[("mask-mode", "luminance")],
        "mask-match" => &[("mask-mode", "match-source")],
        "mask-type-alpha" => &[("mask-type", "alpha")],
        "mask-type-luminance" => &[("mask-type", "luminance")],
        "mask-auto" => &[("mask-size", "auto")],
        "mask-cover" => &[("mask-size", "cover")],
        "mask-contain" => &[("mask-size", "contain")],
        "mask-top" => &[("mask-position", "top")],
        "mask-top-left" => &[("mask-position", "left top")],
        "mask-top-right" => &[("mask-position", "right top")],
        "mask-bottom" => &[("mask-position", "bottom")],
        "mask-bottom-left" => &[("mask-position", "left bottom")],
        "mask-bottom-right" => &[("mask-position", "right bottom")],
        "mask-left" => &[("mask-position", "left")],
        "mask-right" => &[("mask-position", "right")],
        "mask-center" => &[("mask-position", "center")],
        "mask-repeat" => &[("mask-repeat", "repeat")],
        "mask-no-repeat" => &[("mask-repeat", "no-repeat")],
        "mask-repeat-x" => &[("mask-repeat", "repeat-x")],
        "mask-repeat-y" => &[("mask-repeat", "repeat-y")],
        "mask-repeat-round" => &[("mask-repeat", "round")],
        "mask-repeat-space" => &[("mask-repeat", "space")],
        "mask-clip-border" => &[("mask-clip", "border-box")],
        "mask-clip-padding" => &[("mask-clip", "padding-box")],
        "mask-clip-content" => &[("mask-clip", "content-box")],
        "mask-clip-fill" => &[("mask-clip", "fill-box")],
        "mask-clip-stroke" => &[("mask-clip", "stroke-box")],
        "mask-clip-view" => &[("mask-clip", "view-box")],
        "mask-no-clip" => &[("mask-clip", "no-clip")],
        "mask-origin-border" => &[("mask-origin", "border-box")],
        "mask-origin-padding" => &[("mask-origin", "padding-box")],
        "mask-origin-content" => &[("mask-origin", "content-box")],
        "mask-origin-fill" => &[("mask-origin", "fill-box")],
        "mask-origin-stroke" => &[("mask-origin", "stroke-box")],
        "mask-origin-view" => &[("mask-origin", "view-box")],
        "perspective-origin-center" => &[("perspective-origin", "center")],
        "perspective-origin-top" => &[("perspective-origin", "top")],
        "perspective-origin-top-right" => &[("perspective-origin", "100% 0")],
        "perspective-origin-right" => &[("perspective-origin", "100%")],
        "perspective-origin-bottom-right" => &[("perspective-origin", "100% 100%")],
        "perspective-origin-bottom" => &[("perspective-origin", "bottom")],
        "perspective-origin-bottom-left" => &[("perspective-origin", "0 100%")],
        "perspective-origin-left" => &[("perspective-origin", "0")],
        "perspective-origin-top-left" => &[("perspective-origin", "0 0")],
        "line-clamp-none" => &[
            ("overflow", "visible"),
            ("display", "block"),
            ("-webkit-box-orient", "horizontal"),
            ("-webkit-line-clamp", "unset"),
        ],
        "outline-solid" => &[("--tw-outline-style", "solid"), ("outline-style", "solid")],
        "outline-dashed" => &[
            ("--tw-outline-style", "dashed"),
            ("outline-style", "dashed"),
        ],
        "outline-dotted" => &[
            ("--tw-outline-style", "dotted"),
            ("outline-style", "dotted"),
        ],
        "outline-double" => &[
            ("--tw-outline-style", "double"),
            ("outline-style", "double"),
        ],
        "outline-none" => &[("--tw-outline-style", "none"), ("outline-style", "none")],
        "font-stretch-normal" => &[("font-stretch", "normal")],
        "font-stretch-ultra-condensed" => &[("font-stretch", "ultra-condensed")],
        "font-stretch-extra-condensed" => &[("font-stretch", "extra-condensed")],
        "font-stretch-condensed" => &[("font-stretch", "condensed")],
        "font-stretch-semi-condensed" => &[("font-stretch", "semi-condensed")],
        "font-stretch-semi-expanded" => &[("font-stretch", "semi-expanded")],
        "font-stretch-expanded" => &[("font-stretch", "expanded")],
        "font-stretch-extra-expanded" => &[("font-stretch", "extra-expanded")],
        "font-stretch-ultra-expanded" => &[("font-stretch", "ultra-expanded")],
        _ => return None,
    })
}

/// The border styles `border-dashed` and the like set, which `border-<width>`
/// draws in
const BORDER_STYLES: [&str; 6] = ["solid", "dashed", "dotted", "double", "hidden", "none"];

/// The sides of `border-<side>-<width>`: the property each side's style and
/// width are in
fn border_side(side: &str) -> Option<(&'static str, &'static str)> {
    Some(match side {
        "" => ("border-style", "border-width"),
        "x" => ("border-inline-style", "border-inline-width"),
        "y" => ("border-block-style", "border-block-width"),
        "s" => ("border-inline-start-style", "border-inline-start-width"),
        "e" => ("border-inline-end-style", "border-inline-end-width"),
        "t" => ("border-top-style", "border-top-width"),
        "r" => ("border-right-style", "border-right-width"),
        "b" => ("border-bottom-style", "border-bottom-width"),
        "l" => ("border-left-style", "border-left-width"),
        _ => return None,
    })
}

/// A number of pixels written as a bare number, or an arbitrary value
fn pixels_or_value(argument: &str) -> Option<String> {
    if is_positive_integer(argument) {
        return Some(format!("{argument}px"));
    }
    arbitrary_or_variable(argument)
}

/// A width in pixels written as a bare number, or as an arbitrary length: what
/// `border-[3px]` is, while `border-[red]` is a color
fn pixels(argument: &str) -> Option<String> {
    if is_positive_integer(argument) {
        return Some(format!("{argument}px"));
    }
    arbitrary_or_variable(argument)
        .filter(|_| argument.starts_with('['))
        .filter(|value| value_type(None, value) == Some(ValueType::Length))
}

/// `border`, `border-2`, `border-x-4` and `border-dashed`: a width drawn in the
/// style `--tw-border-style` holds
fn border_utility(name: &str) -> Option<Vec<Declaration>> {
    let rest = name.strip_prefix("border")?;
    let rest = if rest.is_empty() {
        rest
    } else {
        rest.strip_prefix('-')?
    };
    if let Some(style) = Some(rest).filter(|style| BORDER_STYLES.contains(style)) {
        return Some(vec![
            decl("--tw-border-style", style.to_string()),
            decl("border-style", style.to_string()),
        ]);
    }
    let (side, argument) = match rest.split_once('-') {
        Some((side, argument)) if border_side(side).is_some() => (side, Some(argument)),
        None if border_side(rest).is_some() => (rest, None),
        _ => ("", Some(rest)),
    };
    let (style, property) = border_side(side)?;
    let width = argument.map_or_else(|| Some(String::from("1px")), pixels)?;
    Some(vec![
        decl(style, "var(--tw-border-style)"),
        decl(property, width),
    ])
}

/// `outline` and `outline-2`: a width drawn in the style `--tw-outline-style`
/// holds
fn outline_width(name: &str) -> Option<Vec<Declaration>> {
    let width = match name.strip_prefix("outline")? {
        "" => String::from("1px"),
        argument => pixels(argument.strip_prefix('-')?)?,
    };
    Some(vec![
        decl("outline-style", "var(--tw-outline-style)"),
        decl("outline-width", width),
    ])
}

/// `border-spacing-*`, which sets both axes unless one is named
fn border_spacing(name: &str) -> Option<Vec<Declaration>> {
    let rest = name.strip_prefix("border-spacing-")?;
    let (axes, argument): (&[&str], &str) = match rest.split_once('-') {
        Some(("x", argument)) => (&["x"], argument),
        Some(("y", argument)) => (&["y"], argument),
        _ => (&["x", "y"], rest),
    };
    let value = spacing(argument, false)?;
    let mut declarations: Vec<Declaration> = axes
        .iter()
        .map(|axis| decl(format!("--tw-border-spacing-{axis}"), value.clone()))
        .collect();
    declarations.push(decl(
        "border-spacing",
        "var(--tw-border-spacing-x) var(--tw-border-spacing-y)",
    ));
    Some(declarations)
}

/// The logical sides of `start-*` and `end-*`, and the margins and paddings of
/// scrolling `scroll-m*` and `scroll-p*`
fn spaced_properties(root: &str) -> Option<&'static [&'static str]> {
    Some(match root {
        "start" => &["inset-inline-start"],
        "end" => &["inset-inline-end"],
        "scroll-m" => &["scroll-margin"],
        "scroll-mx" => &["scroll-margin-inline"],
        "scroll-my" => &["scroll-margin-block"],
        "scroll-ms" => &["scroll-margin-inline-start"],
        "scroll-me" => &["scroll-margin-inline-end"],
        "scroll-mt" => &["scroll-margin-top"],
        "scroll-mr" => &["scroll-margin-right"],
        "scroll-mb" => &["scroll-margin-bottom"],
        "scroll-ml" => &["scroll-margin-left"],
        "scroll-p" => &["scroll-padding"],
        "scroll-px" => &["scroll-padding-inline"],
        "scroll-py" => &["scroll-padding-block"],
        "scroll-ps" => &["scroll-padding-inline-start"],
        "scroll-pe" => &["scroll-padding-inline-end"],
        "scroll-pt" => &["scroll-padding-top"],
        "scroll-pr" => &["scroll-padding-right"],
        "scroll-pb" => &["scroll-padding-bottom"],
        "scroll-pl" => &["scroll-padding-left"],
        "indent" => &["text-indent"],
        _ => return None,
    })
}

/// A number of pixels, negated
fn negated_pixels(argument: &str) -> Option<String> {
    Some(format!("calc({} * -1)", pixels_or_value(argument)?))
}

/// What `font-variant-numeric` holds: every figure, spacing and fraction the
/// numeric utilities set
const NUMERIC: &str = "var(--tw-ordinal,) var(--tw-slashed-zero,) var(--tw-numeric-figure,) var(--tw-numeric-spacing,) var(--tw-numeric-fraction,)";

/// What `contain` holds when `contain-size` and its like compose
const CONTAIN: &str = "var(--tw-contain-size,) var(--tw-contain-layout,) var(--tw-contain-paint,) var(--tw-contain-style,)";

/// The composed `font-variant-numeric` and `contain` utilities: the variable
/// each sets, and its value
fn composed(name: &str) -> Option<Vec<Declaration>> {
    let (variable, value, property, shared) = match name {
        "ordinal" => ("--tw-ordinal", "ordinal", "font-variant-numeric", NUMERIC),
        "slashed-zero" => (
            "--tw-slashed-zero",
            "slashed-zero",
            "font-variant-numeric",
            NUMERIC,
        ),
        "lining-nums" => (
            "--tw-numeric-figure",
            "lining-nums",
            "font-variant-numeric",
            NUMERIC,
        ),
        "oldstyle-nums" => (
            "--tw-numeric-figure",
            "oldstyle-nums",
            "font-variant-numeric",
            NUMERIC,
        ),
        "proportional-nums" => (
            "--tw-numeric-spacing",
            "proportional-nums",
            "font-variant-numeric",
            NUMERIC,
        ),
        "tabular-nums" => (
            "--tw-numeric-spacing",
            "tabular-nums",
            "font-variant-numeric",
            NUMERIC,
        ),
        "diagonal-fractions" => (
            "--tw-numeric-fraction",
            "diagonal-fractions",
            "font-variant-numeric",
            NUMERIC,
        ),
        "stacked-fractions" => (
            "--tw-numeric-fraction",
            "stacked-fractions",
            "font-variant-numeric",
            NUMERIC,
        ),
        "contain-size" => ("--tw-contain-size", "size", "contain", CONTAIN),
        "contain-inline-size" => ("--tw-contain-size", "inline-size", "contain", CONTAIN),
        "contain-layout" => ("--tw-contain-layout", "layout", "contain", CONTAIN),
        "contain-paint" => ("--tw-contain-paint", "paint", "contain", CONTAIN),
        "contain-style" => ("--tw-contain-style", "style", "contain", CONTAIN),
        _ => return None,
    };
    Some(vec![decl(variable, value), decl(property, shared)])
}

/// `rotate-x-45`, `translate-z-4`, `scale-z-50`: the 3D steps of the transform
/// utilities, set in their own variable and composed with the others
fn three_d(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    if let Some(argument) = name.strip_prefix("rotate-") {
        let (axis, argument) = argument.split_once('-')?;
        if !matches!(axis, "x" | "y" | "z") {
            return None;
        }
        let value = if is_positive_integer(argument) {
            format!("{argument}deg")
        } else {
            arbitrary_or_variable(argument)?
        };
        let value = if negative {
            format!("calc({value} * -1)")
        } else {
            value
        };
        return Some(vec![
            decl(
                format!("--tw-rotate-{axis}"),
                format!("rotate{}({value})", axis.to_ascii_uppercase()),
            ),
            decl("transform", TRANSFORM),
        ]);
    }
    if let Some(argument) = name.strip_prefix("translate-z-") {
        return Some(vec![
            decl("--tw-translate-z", spacing(argument, negative)?),
            decl(
                "translate",
                "var(--tw-translate-x) var(--tw-translate-y) var(--tw-translate-z)",
            ),
        ]);
    }
    let argument = name.strip_prefix("scale-z-")?;
    let value = if is_positive_integer(argument) {
        format!("{argument}%")
    } else {
        arbitrary_or_variable(argument)?
    };
    let value = if negative {
        format!("calc({value} * -1)")
    } else {
        value
    };
    Some(vec![
        decl("--tw-scale-z", value),
        decl(
            "scale",
            "var(--tw-scale-x) var(--tw-scale-y) var(--tw-scale-z)",
        ),
    ])
}

/// `perspective-near`, `perspective-[300px]` and `perspective-origin-[10%_20%]`
fn perspective(name: &str) -> Option<Vec<Declaration>> {
    if let Some(argument) = name.strip_prefix("perspective-origin-") {
        return Some(vec![decl(
            "perspective-origin",
            arbitrary_or_variable(argument)?,
        )]);
    }
    let argument = name.strip_prefix("perspective-")?;
    if argument == "none" {
        return Some(vec![decl("perspective", "none")]);
    }
    let value = themed("perspective", argument, |key| {
        Some(match key {
            "dramatic" => "100px",
            "near" => "300px",
            "normal" => "500px",
            "midrange" => "800px",
            "distant" => "1200px",
            _ => return None,
        })
    })
    .map(Cow::into_owned)
    .or_else(|| arbitrary_or_variable(argument))?;
    Some(vec![decl("perspective", value)])
}

/// `mask-[...]`, `mask-size-[...]` and `mask-position-[...]`
fn mask(name: &str) -> Option<Vec<Declaration>> {
    if let Some(argument) = name.strip_prefix("mask-size-") {
        return Some(vec![decl("mask-size", arbitrary_or_variable(argument)?)]);
    }
    if let Some(argument) = name.strip_prefix("mask-position-") {
        return Some(vec![decl(
            "mask-position",
            arbitrary_or_variable(argument)?,
        )]);
    }
    let value = arbitrary_or_variable(name.strip_prefix("mask-")?)?;
    let percentage = value
        .strip_suffix('%')
        .is_some_and(|number| number.parse::<f64>().is_ok());
    let property = if percentage {
        "mask-position"
    } else if value_type(None, &value) == Some(ValueType::Length) {
        "mask-size"
    } else {
        "mask-image"
    };
    Some(vec![decl(property, value)])
}

/// The utilities that also have declarations in an at-rule: `outline-hidden`
pub fn nested_utility(name: &str) -> Option<(Vec<Declaration>, Vec<Nested>)> {
    (name == "outline-hidden").then(|| {
        (
            vec![
                decl("--tw-outline-style", "none"),
                decl("outline-style", "none"),
            ],
            vec![Nested {
                kind: AtRuleKind::Media,
                query: "(forced-colors: active)",
                declarations: vec![
                    decl("outline", "2px solid transparent"),
                    decl("outline-offset", "2px"),
                ],
            }],
        )
    })
}

/// `@container`, `@container-normal` and `@container/main`: an element whose
/// size the container queries of its descendants read
fn container_utility(name: &str) -> Option<Vec<Declaration>> {
    let rest = name.strip_prefix("@container")?;
    let parts = split_top_level(rest, '/');
    let (kind, container) = match parts.as_slice() {
        [kind] => (*kind, None),
        [kind, container] if !container.is_empty() => (*kind, Some(*container)),
        _ => return None,
    };
    let value = match kind {
        "" => String::from("inline-size"),
        "-normal" => String::from("normal"),
        "-size" => String::from("size"),
        _ => bracketed(kind.strip_prefix('-')?).map(decode_arbitrary_value)?,
    };
    let mut declarations = vec![decl("container-type", value)];
    if let Some(container) = container {
        declarations.push(decl("container-name", container.to_string()));
    }
    Some(declarations)
}

/// The declarations of the utilities of this module, `None` for any other
pub fn misc_utility(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    if negative {
        return negated(name);
    }
    if let Some(declarations) = container_utility(name) {
        return Some(declarations);
    }
    if let Some(declarations) = fixed(name) {
        return Some(
            declarations
                .iter()
                .map(|&(property, value)| decl(property, value))
                .collect(),
        );
    }
    if let Some(declarations) = composed(name)
        .or_else(|| perspective(name))
        .or_else(|| mask(name))
    {
        return Some(declarations);
    }
    if let Some(declarations) = border_utility(name)
        .or_else(|| border_spacing(name))
        .or_else(|| outline_width(name))
    {
        return Some(declarations);
    }
    three_d(name, false).or_else(|| plain(name))
}

/// The properties of a spaced utility such as `scroll-mt-2` and its argument
fn split_spaced(name: &str) -> Option<(&'static [&'static str], &str)> {
    name.match_indices('-').find_map(|(index, _)| {
        spaced_properties(&name[..index]).map(|properties| (properties, &name[index + 1..]))
    })
}

/// The utilities a leading `-` negates
fn negated(name: &str) -> Option<Vec<Declaration>> {
    if let Some(declarations) = three_d(name, true) {
        return Some(declarations);
    }
    if let Some(argument) = name.strip_prefix("underline-offset-") {
        return Some(vec![decl(
            "text-underline-offset",
            negated_pixels(argument)?,
        )]);
    }
    if let Some(argument) = name.strip_prefix("outline-offset-") {
        return Some(vec![decl("outline-offset", negated_pixels(argument)?)]);
    }
    let (properties, argument) = split_spaced(name)?;
    let value = spacing(argument, true)?;
    Some(
        properties
            .iter()
            .map(|&property| decl(property, value.clone()))
            .collect(),
    )
}
/// The functional utilities: `line-clamp-3`, `underline-offset-4`,
/// `decoration-2`, `decoration-<color>`, `outline-offset-2`, `font-stretch-75%`
/// and the spaced ones
fn plain(name: &str) -> Option<Vec<Declaration>> {
    if let Some(lines) = name.strip_prefix("line-clamp-") {
        let lines = if is_positive_integer(lines) {
            lines.to_string()
        } else {
            arbitrary_or_variable(lines)?
        };
        return Some(vec![
            decl("overflow", "hidden"),
            decl("display", "-webkit-box"),
            decl("-webkit-box-orient", "vertical"),
            decl("-webkit-line-clamp", lines),
        ]);
    }
    if let Some(argument) = name.strip_prefix("underline-offset-") {
        return Some(vec![decl(
            "text-underline-offset",
            pixels_or_value(argument)?,
        )]);
    }
    if let Some(argument) = name.strip_prefix("outline-offset-") {
        return Some(vec![decl("outline-offset", pixels_or_value(argument)?)]);
    }
    if let Some(argument) = name.strip_prefix("decoration-") {
        if let Some(width) = pixels(argument) {
            return Some(vec![decl("text-decoration-thickness", width)]);
        }
        return Some(vec![decl("text-decoration-color", color_value(argument)?)]);
    }
    if let Some(percent) = name.strip_prefix("font-stretch-") {
        let number = percent.strip_suffix('%')?;
        let valid = is_positive_integer(number)
            && number.parse::<u32>().is_ok_and(|n| (50..=200).contains(&n));
        return valid.then(|| vec![decl("font-stretch", percent.to_string())]);
    }
    let (properties, argument) = split_spaced(name)?;
    let value = spacing_scale(argument)
        .map(String::from)
        .or_else(|| arbitrary_or_variable(argument))?;
    Some(
        properties
            .iter()
            .map(|&property| decl(property, value.clone()))
            .collect(),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use rstest::rstest;

    use crate::tailwind::declarations_of;

    fn conditions(class: &str) -> Vec<String> {
        crate::tailwind::parse_class(class)
            .unwrap()
            .conditions
            .iter()
            .map(|condition| {
                condition
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string)
            })
            .collect()
    }

    fn expect(class: &str, expected: &[(&str, &str)]) {
        let expected: Vec<(String, String)> = expected
            .iter()
            .map(|(property, value)| (property.to_string(), value.to_string()))
            .collect();
        assert_eq!(declarations_of(class), Some(expected), "{class}");
    }

    #[rstest]
    #[case("field-sizing-content", &[("field-sizing", "content")])]
    #[case("table-fixed", &[("table-layout", "fixed")])]
    #[case("caption-bottom", &[("caption-side", "bottom")])]
    #[case("transform-3d", &[("transform-style", "preserve-3d")])]
    #[case("transform-none", &[("transform", "none")])]
    #[case("backface-hidden", &[("backface-visibility", "hidden")])]
    #[case("forced-color-adjust-none", &[("forced-color-adjust", "none")])]
    #[case("antialiased", &[("-webkit-font-smoothing", "antialiased"), ("-moz-osx-font-smoothing", "grayscale")])]
    #[case("subpixel-antialiased", &[("-webkit-font-smoothing", "auto"), ("-moz-osx-font-smoothing", "auto")])]
    #[case("scheme-light-dark", &[("color-scheme", "light dark")])]
    #[case("wrap-anywhere", &[("overflow-wrap", "anywhere")])]
    #[case("decoration-wavy", &[("text-decoration-style", "wavy")])]
    #[case("decoration-from-font", &[("text-decoration-thickness", "from-font")])]
    #[case("underline-offset-auto", &[("text-underline-offset", "auto")])]
    #[case("list-image-none", &[("list-style-image", "none")])]
    #[case("perspective-near", &[("perspective", "300px")])]
    #[case("perspective-none", &[("perspective", "none")])]
    #[case("font-stretch-ultra-condensed", &[("font-stretch", "ultra-condensed")])]
    #[case("font-stretch-75%", &[("font-stretch", "75%")])]
    #[case("outline-solid", &[("--tw-outline-style", "solid"), ("outline-style", "solid")])]
    #[case("outline-dashed", &[("--tw-outline-style", "dashed"), ("outline-style", "dashed")])]
    #[case("outline-dotted", &[("--tw-outline-style", "dotted"), ("outline-style", "dotted")])]
    #[case("outline-double", &[("--tw-outline-style", "double"), ("outline-style", "double")])]
    #[case("outline-none", &[("--tw-outline-style", "none"), ("outline-style", "none")])]
    #[case("line-clamp-none", &[("overflow", "visible"), ("display", "block"), ("-webkit-box-orient", "horizontal"), ("-webkit-line-clamp", "unset")])]
    fn fixed_utilities(#[case] class: &str, #[case] expected: &[(&str, &str)]) {
        expect(class, expected);
    }

    #[rstest]
    #[case("line-clamp-3", "3")]
    #[case("line-clamp-(--lines)", "var(--lines)")]
    #[case("line-clamp-[calc(1+1)]", "calc(1 + 1)")]
    fn line_clamp_clamps_to_a_number_of_lines(#[case] class: &str, #[case] lines: &str) {
        expect(
            class,
            &[
                ("overflow", "hidden"),
                ("display", "-webkit-box"),
                ("-webkit-box-orient", "vertical"),
                ("-webkit-line-clamp", lines),
            ],
        );
    }

    #[test]
    fn text_decoration_offsets_and_outlines() {
        expect("underline-offset-4", &[("text-underline-offset", "4px")]);
        expect(
            "-underline-offset-4",
            &[("text-underline-offset", "calc(4px * -1)")],
        );
        expect(
            "underline-offset-[3px]",
            &[("text-underline-offset", "3px")],
        );
        expect("decoration-2", &[("text-decoration-thickness", "2px")]);
        expect("decoration-[3px]", &[("text-decoration-thickness", "3px")]);
        expect(
            "decoration-red-500",
            &[("text-decoration-color", "oklch(63.7% 0.237 25.331)")],
        );
        expect("outline-offset-2", &[("outline-offset", "2px")]);
        expect("-outline-offset-2", &[("outline-offset", "calc(2px * -1)")]);
        expect("outline-offset-(--o)", &[("outline-offset", "var(--o)")]);
        expect(
            "outline",
            &[
                ("outline-style", "var(--tw-outline-style)"),
                ("outline-width", "1px"),
            ],
        );
        expect(
            "outline-4",
            &[
                ("outline-style", "var(--tw-outline-style)"),
                ("outline-width", "4px"),
            ],
        );
        expect(
            "outline-[3px]",
            &[
                ("outline-style", "var(--tw-outline-style)"),
                ("outline-width", "3px"),
            ],
        );
    }

    #[rstest]
    #[case("border", "border-style", "border-width", "1px")]
    #[case("border-0", "border-style", "border-width", "0px")]
    #[case("border-2", "border-style", "border-width", "2px")]
    #[case("border-[3px]", "border-style", "border-width", "3px")]
    #[case("border-x", "border-inline-style", "border-inline-width", "1px")]
    #[case("border-y-4", "border-block-style", "border-block-width", "4px")]
    #[case(
        "border-s-2",
        "border-inline-start-style",
        "border-inline-start-width",
        "2px"
    )]
    #[case(
        "border-e",
        "border-inline-end-style",
        "border-inline-end-width",
        "1px"
    )]
    #[case("border-t", "border-top-style", "border-top-width", "1px")]
    #[case("border-r-8", "border-right-style", "border-right-width", "8px")]
    #[case("border-b-[2px]", "border-bottom-style", "border-bottom-width", "2px")]
    #[case("border-l-2", "border-left-style", "border-left-width", "2px")]
    fn border_widths_draw_in_the_border_style(
        #[case] class: &str,
        #[case] style: &str,
        #[case] width_property: &str,
        #[case] width: &str,
    ) {
        expect(
            class,
            &[(style, "var(--tw-border-style)"), (width_property, width)],
        );
    }

    #[test]
    fn border_styles_set_the_style_the_widths_draw_in() {
        for style in ["solid", "dashed", "dotted", "double", "hidden", "none"] {
            expect(
                &format!("border-{style}"),
                &[("--tw-border-style", style), ("border-style", style)],
            );
        }
    }

    #[test]
    fn border_spacing_sets_both_axes_unless_one_is_named() {
        expect(
            "border-spacing-2",
            &[
                ("--tw-border-spacing-x", "0.5rem"),
                ("--tw-border-spacing-y", "0.5rem"),
                (
                    "border-spacing",
                    "var(--tw-border-spacing-x) var(--tw-border-spacing-y)",
                ),
            ],
        );
        expect(
            "border-spacing-x-4",
            &[
                ("--tw-border-spacing-x", "1rem"),
                (
                    "border-spacing",
                    "var(--tw-border-spacing-x) var(--tw-border-spacing-y)",
                ),
            ],
        );
        expect(
            "border-spacing-y-[3px]",
            &[
                ("--tw-border-spacing-y", "3px"),
                (
                    "border-spacing",
                    "var(--tw-border-spacing-x) var(--tw-border-spacing-y)",
                ),
            ],
        );
    }

    #[test]
    fn logical_sides_scroll_spacing_and_indent() {
        expect("start-4", &[("inset-inline-start", "1rem")]);
        expect("end-0", &[("inset-inline-end", "0px")]);
        expect("-start-4", &[("inset-inline-start", "-1rem")]);
        expect("scroll-mt-2", &[("scroll-margin-top", "0.5rem")]);
        expect("scroll-mx-[3px]", &[("scroll-margin-inline", "3px")]);
        expect("-scroll-ms-4", &[("scroll-margin-inline-start", "-1rem")]);
        expect("scroll-pb-4", &[("scroll-padding-bottom", "1rem")]);
        expect("scroll-p-2", &[("scroll-padding", "0.5rem")]);
        expect("indent-8", &[("text-indent", "2rem")]);
        expect("-indent-px", &[("text-indent", "-1px")]);
        expect("indent-(--i)", &[("text-indent", "var(--i)")]);
        for (class, property) in [
            ("scroll-m-1", "scroll-margin"),
            ("scroll-my-1", "scroll-margin-block"),
            ("scroll-me-1", "scroll-margin-inline-end"),
            ("scroll-mr-1", "scroll-margin-right"),
            ("scroll-mb-1", "scroll-margin-bottom"),
            ("scroll-ml-1", "scroll-margin-left"),
            ("scroll-px-1", "scroll-padding-inline"),
            ("scroll-py-1", "scroll-padding-block"),
            ("scroll-ps-1", "scroll-padding-inline-start"),
            ("scroll-pe-1", "scroll-padding-inline-end"),
            ("scroll-pt-1", "scroll-padding-top"),
            ("scroll-pr-1", "scroll-padding-right"),
            ("scroll-pl-1", "scroll-padding-left"),
        ] {
            expect(class, &[(property, "0.25rem")]);
        }
    }

    #[rstest]
    #[case("borderx-2")]
    #[case("border-x-")]
    #[case("border-2.5")]
    #[case("border-02")]
    #[case("border-[url(x)]")]
    #[case("border-spacing-")]
    #[case("border-spacing-x-")]
    #[case("border-spacing-huge")]
    #[case("-border-2")]
    #[case("outline-")]
    #[case("outlinex")]
    #[case("outline-nothing")]
    #[case("outline-offset-")]
    #[case("-outline-offset-huge")]
    #[case("underline-offset-")]
    #[case("-underline-offset-1.5")]
    #[case("line-clamp-")]
    #[case("line-clamp-1.5")]
    #[case("decoration-")]
    #[case("decoration-nothing")]
    #[case("font-stretch-49%")]
    #[case("font-stretch-201%")]
    #[case("font-stretch-75")]
    #[case("font-stretch-1.5%")]
    #[case("font-stretch-normal-1")]
    #[case("start-")]
    #[case("start-huge")]
    #[case("-start-huge")]
    #[case("scroll-mq-4")]
    #[case("-scroll-m-")]
    #[case("-indent")]
    #[case("-table-auto")]
    #[case("antialiased-")]
    fn unknown_utilities_stay_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    #[rstest]
    #[case("@sm:p-4", "@container(width>=24rem)")]
    #[case("@md:p-4", "@container(width>=28rem)")]
    #[case("@3xs:p-4", "@container(width>=16rem)")]
    #[case("@7xl:p-4", "@container(width>=80rem)")]
    #[case("@max-md:p-4", "@container(width<28rem)")]
    #[case("@min-lg:p-4", "@container(width>=32rem)")]
    #[case("@min-[400px]:p-4", "@container(width>=400px)")]
    #[case("@max-[30rem]:p-4", "@container(width<30rem)")]
    #[case("@[30rem]:p-4", "@container(width>=30rem)")]
    #[case("@sm/main:p-4", "@container main (width>=24rem)")]
    #[case("@max-md/sidebar:p-4", "@container sidebar (width<28rem)")]
    fn container_queries_style_by_the_width_of_the_container(
        #[case] class: &str,
        #[case] condition: &str,
    ) {
        assert_eq!(conditions(class), vec![condition]);
    }

    #[rstest]
    #[case("@sm/:p-4")]
    #[case("@huge:p-4")]
    #[case("@sm/a/b:p-4")]
    #[case("@:p-4")]
    #[case("@max-:p-4")]
    #[case("@[]:p-4")]
    #[case("@[var(--w)]:p-4")]
    #[case("@max-[calc(var(--w))]:p-4")]
    fn unknown_container_queries_stay_as_written(#[case] class: &str) {
        assert_eq!(crate::tailwind::parse_class(class), None, "{class}");
    }

    #[test]
    fn container_queries_nest_with_other_variants() {
        assert_eq!(
            conditions("hover:@md:p-4"),
            vec!["@media(hover:hover) @container(width>=28rem) &:hover"]
        );
        assert_eq!(
            conditions("@md:hover:p-4"),
            vec!["@container(width>=28rem) @media(hover:hover) &:hover"]
        );
    }

    #[rstest]
    #[case("@container", &[("container-type", "inline-size")])]
    #[case("@container-normal", &[("container-type", "normal")])]
    #[case("@container-size", &[("container-type", "size")])]
    #[case("@container-[inline-size_layout]", &[("container-type", "inline-size layout")])]
    #[case("@container/main", &[("container-type", "inline-size"), ("container-name", "main")])]
    #[case("@container-normal/main", &[("container-type", "normal"), ("container-name", "main")])]
    fn container_utilities_make_the_element_a_container(
        #[case] class: &str,
        #[case] expected: &[(&str, &str)],
    ) {
        expect(class, expected);
    }

    #[rstest]
    #[case("@container-")]
    #[case("@container-huge")]
    #[case("@container/")]
    #[case("@container/a/b")]
    #[case("@containers")]
    fn unknown_containers_stay_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    #[test]
    fn outline_hidden_has_a_forced_colors_fallback() {
        let class = crate::tailwind::parse_class("outline-hidden").unwrap();
        assert_eq!(
            class.declarations,
            vec![
                ("--tw-outline-style".into(), "none".into()),
                ("outline-style".into(), "none".into()),
            ]
        );
        assert_eq!(class.nested.len(), 1);
        assert_eq!(
            class.nested[0].0[0]
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("@media(forced-colors:active)")
        );
        assert_eq!(
            class.nested[0].1,
            vec![
                ("outline".into(), "2px solid transparent".into()),
                ("outline-offset".into(), "2px".into()),
            ]
        );
        let styles: Vec<_> = class
            .styles()
            .map(|style| style.property().to_string())
            .collect();
        assert_eq!(
            styles,
            vec![
                "--tw-outline-style",
                "outline-style",
                "outline",
                "outline-offset"
            ]
        );
        let important = crate::tailwind::parse_class("outline-hidden!").unwrap();
        assert_eq!(important.nested[0].1[1].1, "2px !important");
        let hovered = crate::tailwind::parse_class("hover:outline-hidden").unwrap();
        assert_eq!(
            hovered.nested[0].0[0]
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("@media(hover:hover)and (forced-colors:active) &:hover")
        );
    }

    #[rstest]
    #[case("ordinal", "--tw-ordinal", "ordinal")]
    #[case("slashed-zero", "--tw-slashed-zero", "slashed-zero")]
    #[case("lining-nums", "--tw-numeric-figure", "lining-nums")]
    #[case("oldstyle-nums", "--tw-numeric-figure", "oldstyle-nums")]
    #[case("proportional-nums", "--tw-numeric-spacing", "proportional-nums")]
    #[case("tabular-nums", "--tw-numeric-spacing", "tabular-nums")]
    #[case("diagonal-fractions", "--tw-numeric-fraction", "diagonal-fractions")]
    #[case("stacked-fractions", "--tw-numeric-fraction", "stacked-fractions")]
    fn numeric_figures_compose_through_their_variables(
        #[case] class: &str,
        #[case] variable: &str,
        #[case] value: &str,
    ) {
        expect(
            class,
            &[
                (variable, value),
                (
                    "font-variant-numeric",
                    "var(--tw-ordinal,) var(--tw-slashed-zero,) var(--tw-numeric-figure,) var(--tw-numeric-spacing,) var(--tw-numeric-fraction,)",
                ),
            ],
        );
        expect("normal-nums", &[("font-variant-numeric", "normal")]);
    }

    #[rstest]
    #[case("contain-size", "--tw-contain-size", "size")]
    #[case("contain-inline-size", "--tw-contain-size", "inline-size")]
    #[case("contain-layout", "--tw-contain-layout", "layout")]
    #[case("contain-paint", "--tw-contain-paint", "paint")]
    #[case("contain-style", "--tw-contain-style", "style")]
    fn contain_composes_through_its_variables(
        #[case] class: &str,
        #[case] variable: &str,
        #[case] value: &str,
    ) {
        expect(
            class,
            &[
                (variable, value),
                (
                    "contain",
                    "var(--tw-contain-size,) var(--tw-contain-layout,) var(--tw-contain-paint,) var(--tw-contain-style,)",
                ),
            ],
        );
    }

    #[test]
    fn contain_keywords() {
        expect("contain-none", &[("contain", "none")]);
        expect("contain-content", &[("contain", "content")]);
        expect("contain-strict", &[("contain", "strict")]);
        expect(
            "transform-cpu",
            &[(
                "transform",
                "var(--tw-rotate-x,) var(--tw-rotate-y,) var(--tw-rotate-z,) var(--tw-skew-x,) var(--tw-skew-y,)",
            )],
        );
        expect(
            "transform-gpu",
            &[(
                "transform",
                "translateZ(0) var(--tw-rotate-x,) var(--tw-rotate-y,) var(--tw-rotate-z,) var(--tw-skew-x,) var(--tw-skew-y,)",
            )],
        );
    }

    #[test]
    fn three_d_transforms_compose_with_the_others() {
        let transform = "var(--tw-rotate-x,) var(--tw-rotate-y,) var(--tw-rotate-z,) var(--tw-skew-x,) var(--tw-skew-y,)";
        expect(
            "rotate-x-45",
            &[
                ("--tw-rotate-x", "rotateX(45deg)"),
                ("transform", transform),
            ],
        );
        expect(
            "rotate-y-12",
            &[
                ("--tw-rotate-y", "rotateY(12deg)"),
                ("transform", transform),
            ],
        );
        expect(
            "rotate-z-0",
            &[("--tw-rotate-z", "rotateZ(0deg)"), ("transform", transform)],
        );
        expect(
            "-rotate-x-45",
            &[
                ("--tw-rotate-x", "rotateX(calc(45deg * -1))"),
                ("transform", transform),
            ],
        );
        expect(
            "rotate-y-[1.5rad]",
            &[
                ("--tw-rotate-y", "rotateY(1.5rad)"),
                ("transform", transform),
            ],
        );
        let translate = "var(--tw-translate-x) var(--tw-translate-y) var(--tw-translate-z)";
        expect(
            "translate-z-4",
            &[("--tw-translate-z", "1rem"), ("translate", translate)],
        );
        expect(
            "-translate-z-4",
            &[("--tw-translate-z", "-1rem"), ("translate", translate)],
        );
        expect(
            "translate-z-[3px]",
            &[("--tw-translate-z", "3px"), ("translate", translate)],
        );
        expect("translate-3d", &[("translate", translate)]);
        let scale = "var(--tw-scale-x) var(--tw-scale-y) var(--tw-scale-z)";
        expect("scale-z-50", &[("--tw-scale-z", "50%"), ("scale", scale)]);
        expect(
            "-scale-z-50",
            &[("--tw-scale-z", "calc(50% * -1)"), ("scale", scale)],
        );
        expect(
            "scale-z-[1.5]",
            &[("--tw-scale-z", "1.5"), ("scale", scale)],
        );
        expect("scale-3d", &[("scale", scale)]);
    }

    #[test]
    fn perspective_and_its_origin() {
        expect("perspective-dramatic", &[("perspective", "100px")]);
        expect("perspective-near", &[("perspective", "300px")]);
        expect("perspective-normal", &[("perspective", "500px")]);
        expect("perspective-midrange", &[("perspective", "800px")]);
        expect("perspective-distant", &[("perspective", "1200px")]);
        expect("perspective-none", &[("perspective", "none")]);
        expect("perspective-[400px]", &[("perspective", "400px")]);
        expect("perspective-(--p)", &[("perspective", "var(--p)")]);
        expect(
            "perspective-origin-[10%_20%]",
            &[("perspective-origin", "10% 20%")],
        );
        for (name, value) in [
            ("center", "center"),
            ("top", "top"),
            ("top-right", "100% 0"),
            ("right", "100%"),
            ("bottom-right", "100% 100%"),
            ("bottom", "bottom"),
            ("bottom-left", "0 100%"),
            ("left", "0"),
            ("top-left", "0 0"),
        ] {
            expect(
                &format!("perspective-origin-{name}"),
                &[("perspective-origin", value)],
            );
        }
    }

    #[test]
    fn masks() {
        expect("mask-none", &[("mask-image", "none")]);
        expect("mask-add", &[("mask-composite", "add")]);
        expect("mask-intersect", &[("mask-composite", "intersect")]);
        expect("mask-luminance", &[("mask-mode", "luminance")]);
        expect("mask-match", &[("mask-mode", "match-source")]);
        expect("mask-type-alpha", &[("mask-type", "alpha")]);
        expect("mask-cover", &[("mask-size", "cover")]);
        expect("mask-top-left", &[("mask-position", "left top")]);
        expect("mask-center", &[("mask-position", "center")]);
        expect("mask-repeat-round", &[("mask-repeat", "round")]);
        expect("mask-clip-padding", &[("mask-clip", "padding-box")]);
        expect("mask-no-clip", &[("mask-clip", "no-clip")]);
        expect("mask-origin-view", &[("mask-origin", "view-box")]);
        expect("mask-[url(/m.svg)]", &[("mask-image", "url(/m.svg)")]);
        expect(
            "mask-[linear-gradient(black,transparent)]",
            &[("mask-image", "linear-gradient(black,transparent)")],
        );
        expect("mask-[50%]", &[("mask-position", "50%")]);
        expect("mask-[10px]", &[("mask-size", "10px")]);
        expect("mask-size-[10px_20px]", &[("mask-size", "10px 20px")]);
        expect("mask-position-(--p)", &[("mask-position", "var(--p)")]);
        for class in [
            "mask-linear-50",
            "mask-t-from-50%",
            "mask-radial-from-50%",
            "mask-conic-from-50",
            "mask-[]",
            "mask-size-",
        ] {
            assert_eq!(declarations_of(class), None, "{class}");
        }
    }
}
