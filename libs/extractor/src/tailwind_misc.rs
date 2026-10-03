//! Utilities of Tailwind CSS v4 that are a fixed set of declarations or one
//! value on a few properties: text decoration and clamping, border widths and
//! styles, spacing on logical sides, table and transform keywords

use crate::tailwind::{
    Declaration, ValueType, arbitrary_or_variable, decl, is_positive_integer, spacing_scale,
    value_type,
};
use crate::tailwind_children::spacing;
use crate::tailwind_color::color_value;

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
        "perspective-none" => &[("perspective", "none")],
        "perspective-dramatic" => &[("perspective", "100px")],
        "perspective-near" => &[("perspective", "300px")],
        "perspective-normal" => &[("perspective", "500px")],
        "perspective-midrange" => &[("perspective", "800px")],
        "perspective-distant" => &[("perspective", "1200px")],
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

/// The declarations of the utilities of this module, `None` for any other
pub fn misc_utility(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    if negative {
        return negated(name);
    }
    if let Some(declarations) = fixed(name) {
        return Some(
            declarations
                .iter()
                .map(|&(property, value)| decl(property, value))
                .collect(),
        );
    }
    if let Some(declarations) = border_utility(name)
        .or_else(|| border_spacing(name))
        .or_else(|| outline_width(name))
    {
        return Some(declarations);
    }
    plain(name)
}

/// The properties of a spaced utility such as `scroll-mt-2` and its argument
fn split_spaced(name: &str) -> Option<(&'static [&'static str], &str)> {
    name.match_indices('-').find_map(|(index, _)| {
        spaced_properties(&name[..index]).map(|properties| (properties, &name[index + 1..]))
    })
}

/// The utilities a leading `-` negates
fn negated(name: &str) -> Option<Vec<Declaration>> {
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
}
