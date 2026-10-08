//! Filters, backdrop filters, shadows and gradients: the Tailwind CSS v4
//! utilities that compose through custom properties, so that `blur-sm` and
//! `grayscale` on one element add up instead of overwriting each other

use std::borrow::Cow;

use css::theme_tokens::is_shadow_token;

use crate::tailwind::{
    Declaration, ValueType, arbitrary_or_variable, decl, decode_arbitrary_value,
    is_positive_integer, split_top_level, value_type,
};
use crate::tailwind_color::{color_value, with_modifier};
use crate::tailwind_theme::themed;

/// What `filter` holds: every function the filter utilities set
const FILTER: &str = "var(--tw-blur,) var(--tw-brightness,) var(--tw-contrast,) var(--tw-grayscale,) var(--tw-hue-rotate,) var(--tw-invert,) var(--tw-saturate,) var(--tw-sepia,) var(--tw-drop-shadow,)";

/// What `backdrop-filter` holds
const BACKDROP_FILTER: &str = "var(--tw-backdrop-blur,) var(--tw-backdrop-brightness,) var(--tw-backdrop-contrast,) var(--tw-backdrop-grayscale,) var(--tw-backdrop-hue-rotate,) var(--tw-backdrop-invert,) var(--tw-backdrop-opacity,) var(--tw-backdrop-saturate,) var(--tw-backdrop-sepia,)";

/// How a bare number after a filter utility reads
#[derive(Clone, Copy)]
enum Bare {
    Percent,
    Degrees,
    Opacity,
    Blur,
}

/// A function the filter utilities set
struct FilterFunction {
    root: &'static str,
    bare: Bare,
    /// The value of the utility without one
    default: Option<&'static str>,
    /// Whether a leading `-` negates it
    negative: bool,
}

static FILTER_FUNCTIONS: [FilterFunction; 9] = [
    FilterFunction {
        root: "blur",
        bare: Bare::Blur,
        default: Some("8px"),
        negative: false,
    },
    FilterFunction {
        root: "brightness",
        bare: Bare::Percent,
        default: None,
        negative: false,
    },
    FilterFunction {
        root: "contrast",
        bare: Bare::Percent,
        default: None,
        negative: false,
    },
    FilterFunction {
        root: "grayscale",
        bare: Bare::Percent,
        default: Some("100%"),
        negative: false,
    },
    FilterFunction {
        root: "hue-rotate",
        bare: Bare::Degrees,
        default: None,
        negative: true,
    },
    FilterFunction {
        root: "invert",
        bare: Bare::Percent,
        default: Some("100%"),
        negative: false,
    },
    FilterFunction {
        root: "opacity",
        bare: Bare::Opacity,
        default: None,
        negative: false,
    },
    FilterFunction {
        root: "saturate",
        bare: Bare::Percent,
        default: None,
        negative: false,
    },
    FilterFunction {
        root: "sepia",
        bare: Bare::Percent,
        default: Some("100%"),
        negative: false,
    },
];

/// The sizes of `blur-*`
fn blur_size(name: &str) -> Option<&'static str> {
    Some(match name {
        "xs" => "4px",
        "sm" => "8px",
        "md" => "12px",
        "lg" => "16px",
        "xl" => "24px",
        "2xl" => "40px",
        "3xl" => "64px",
        _ => return None,
    })
}

/// The value of a bare number such as the `50` of `brightness-50`
fn bare_value(bare: Bare, value: &str) -> Option<String> {
    match bare {
        Bare::Percent if is_positive_integer(value) => Some(format!("{value}%")),
        Bare::Degrees if is_positive_integer(value) => Some(format!("{value}deg")),
        Bare::Opacity if is_opacity_number(value) => Some(format!("{value}%")),
        Bare::Blur => themed("blur", value, blur_size).map(Cow::into_owned),
        _ => None,
    }
}

/// A multiple of 0.25, written without needless zeros
fn is_opacity_number(value: &str) -> bool {
    value
        .parse::<f64>()
        .is_ok_and(|number| number >= 0.0 && number % 0.25 == 0.0 && number.to_string() == value)
}

/// What a filter utility puts in the function: its default, a size, an
/// arbitrary value or a bare number
fn function_value(
    function: &FilterFunction,
    argument: Option<&str>,
    negative: bool,
) -> Option<String> {
    let value = match argument {
        None if function.root == "blur" => themed("blur", "", |_| Some("8px"))?.into_owned(),
        None => function.default?.to_string(),
        Some(argument) => {
            arbitrary_or_variable(argument).or_else(|| bare_value(function.bare, argument))?
        }
    };
    match (negative, function.negative) {
        (false, _) => Some(value),
        (true, true) => Some(format!("calc({value} * -1)")),
        (true, false) => None,
    }
}

/// The declarations of a filter or backdrop filter utility such as `blur-sm`,
/// `backdrop-opacity-50` or `filter-none`
fn filter_utility(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    let (backdrop, name) = name
        .strip_prefix("backdrop-")
        .map_or((false, name), |name| (true, name));
    let composed = if backdrop { BACKDROP_FILTER } else { FILTER };
    let property = if backdrop {
        "backdrop-filter"
    } else {
        "filter"
    };
    let apply = |value: Cow<'static, str>| {
        let mut declarations = Vec::with_capacity(3);
        if backdrop {
            declarations.push(decl("-webkit-backdrop-filter", value.clone()));
        }
        declarations.push(decl(property, value));
        declarations
    };
    if !negative {
        if name == "filter" {
            return Some(apply(Cow::Borrowed(composed)));
        }
        if let Some(argument) = name.strip_prefix("filter-") {
            let value = if argument == "none" {
                Cow::Borrowed("none")
            } else {
                Cow::Owned(arbitrary_or_variable(argument)?)
            };
            return Some(apply(value));
        }
    }
    let function = FILTER_FUNCTIONS
        .iter()
        .filter(|function| backdrop || function.root != "opacity")
        .find(|function| {
            name == function.root
                || name
                    .strip_prefix(function.root)
                    .is_some_and(|rest| rest.starts_with('-'))
        })?;
    let argument = name
        .strip_prefix(function.root)
        .and_then(|rest| rest.strip_prefix('-'));
    let prefix = if backdrop { "--tw-backdrop-" } else { "--tw-" };
    // `blur-none` empties the function, which `var(--tw-blur,)` then skips
    let value = if argument == Some("none") && function.root == "blur" && !negative {
        String::from(" ")
    } else {
        format!(
            "{}({})",
            function.root,
            function_value(function, argument, negative)?
        )
    };
    let mut declarations = vec![decl(format!("{prefix}{}", function.root), value)];
    declarations.extend(apply(Cow::Borrowed(composed)));
    Some(declarations)
}

/// What `box-shadow` holds: the shadows `shadow-*` and `ring-*` set, each
/// nothing until one does
const BOX_SHADOW: &str = "var(--tw-inset-shadow,0 0 #0000), var(--tw-inset-ring-shadow,0 0 #0000), var(--tw-ring-offset-shadow,0 0 #0000), var(--tw-ring-shadow,0 0 #0000), var(--tw-shadow)";

/// The shadows of the default theme, each color to be replaced by `--tw-shadow-color`
fn shadow_size(name: &str) -> Option<&'static str> {
    Some(match name {
        "2xs" => "0 1px rgb(0 0 0 / 0.05)",
        "xs" => "0 1px 2px 0 rgb(0 0 0 / 0.05)",
        "inner" => "inset 0 2px 4px 0 rgb(0 0 0 / 0.05)",
        "" | "sm" => "0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)",
        "md" => "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)",
        "lg" => "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)",
        "xl" => "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)",
        "2xl" => "0 25px 50px -12px rgb(0 0 0 / 0.25)",
        _ => return None,
    })
}

/// Whether `token` of a shadow layer is its color
fn is_shadow_color(token: &str) -> bool {
    const FUNCTIONS: [&str; 10] = [
        "rgb(",
        "rgba(",
        "hsl(",
        "hsla(",
        "hwb(",
        "lab(",
        "lch(",
        "oklab(",
        "oklch(",
        "color-mix(",
    ];
    token.starts_with('#')
        || FUNCTIONS.iter().any(|function| token.starts_with(function))
        || (token.bytes().all(|byte| byte.is_ascii_alphabetic()) && token != "inset")
}

/// `shadow` with the color of each of its layers taken from `--tw-shadow-color`
/// when a `shadow-<color>` utility sets one
fn recolored(shadow: &str, variable: &str) -> String {
    split_top_level(shadow, ',')
        .into_iter()
        .map(|layer| {
            split_top_level(layer.trim(), ' ')
                .into_iter()
                .map(|token| {
                    if is_shadow_color(token) {
                        format!("var({variable}, {token})")
                    } else {
                        token.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(", ")
}
/// The declarations of a `shadow` utility: a size, `none`, a theme shadow or
/// a color
fn shadow_utility(name: &str) -> Option<Vec<Declaration>> {
    let argument = if name == "shadow" {
        ""
    } else {
        name.strip_prefix("shadow-")
            .filter(|argument| !argument.is_empty())?
    };
    let composed =
        |shadow: String| vec![decl("--tw-shadow", shadow), decl("box-shadow", BOX_SHADOW)];
    if let Some(shadow) = themed("shadow", argument, shadow_size) {
        return Some(composed(recolored(&shadow, "--tw-shadow-color")));
    }
    match argument {
        "none" => return Some(composed(String::from("0 0 #0000"))),
        "inherit" => return Some(vec![decl("--tw-shadow-color", "inherit")]),
        _ => {}
    }
    if is_shadow_token(argument) {
        return Some(composed(format!("var(--{argument})")));
    }
    let color = shadow_color(argument)?;
    Some(vec![decl(
        "--tw-shadow-color",
        format!("color-mix(in oklab, {color} var(--tw-shadow-alpha), transparent)"),
    )])
}

/// A color written in an argument
enum Argument {
    /// The argument is no arbitrary value
    Named,
    /// An arbitrary value that is a color
    Color(Cow<'static, str>),
    /// An arbitrary value that is not, or a modifier that cannot be applied
    Invalid,
}

/// The color of `shadow-<color>`, a named one or an arbitrary one
fn shadow_color(argument: &str) -> Option<Cow<'static, str>> {
    match arbitrary_color(argument) {
        Argument::Named => color_value(argument),
        Argument::Color(color) => Some(color),
        Argument::Invalid => None,
    }
}

/// `[value]` or `[value]/50` read as a color
fn arbitrary_color(argument: &str) -> Argument {
    let parts = split_top_level(argument, '/');
    let Some(value) = parts[0]
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Argument::Named;
    };
    let value = decode_arbitrary_value(value);
    let single = !value.is_empty() && split_top_level(&value, ' ').len() == 1;
    if !single || value_type(None, &value) != Some(ValueType::Color) {
        return Argument::Invalid;
    }
    let modifier = match parts[1..] {
        [] => None,
        [modifier] => Some(modifier),
        _ => return Argument::Invalid,
    };
    with_modifier(&value, modifier).map_or(Argument::Invalid, Argument::Color)
}
/// The drop shadows of the default theme
fn drop_shadow_size(name: &str) -> Option<&'static str> {
    Some(match name {
        "xs" => "0 1px 1px rgb(0 0 0 / 0.05)",
        "sm" => "0 1px 2px rgb(0 0 0 / 0.15)",
        "md" => "0 3px 3px rgb(0 0 0 / 0.12)",
        "lg" => "0 4px 4px rgb(0 0 0 / 0.15)",
        "xl" => "0 9px 7px rgb(0 0 0 / 0.1)",
        "2xl" => "0 25px 25px rgb(0 0 0 / 0.15)",
        "" => "0 1px 2px rgb(0 0 0 / 0.1), 0 1px 1px rgb(0 0 0 / 0.06)",
        _ => return None,
    })
}

/// `shadow`, a list of shadows, as the `drop-shadow()` functions of `filter`
fn drop_shadows(shadow: &str) -> String {
    split_top_level(shadow, ',')
        .into_iter()
        .map(|layer| format!("drop-shadow({})", layer.trim()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The declarations of a `drop-shadow` utility: composed into `filter` with
/// the other filters, its color apart from its size
fn drop_shadow_utility(name: &str) -> Option<Vec<Declaration>> {
    let argument = if name == "drop-shadow" {
        ""
    } else {
        name.strip_prefix("drop-shadow-")
            .filter(|argument| !argument.is_empty())?
    };
    let sized = |size: String, shadow: String| {
        vec![
            decl(
                "--tw-drop-shadow-size",
                drop_shadows(&recolored(&size, "--tw-drop-shadow-color")),
            ),
            decl("--tw-drop-shadow", shadow),
            decl("filter", FILTER),
        ]
    };
    if argument == "none" {
        return Some(vec![decl("--tw-drop-shadow", " "), decl("filter", FILTER)]);
    }
    if argument == "inherit" {
        return Some(vec![
            decl("--tw-drop-shadow-color", "inherit"),
            decl("--tw-drop-shadow", "var(--tw-drop-shadow-size)"),
        ]);
    }
    if let Some(size) = themed("drop-shadow", argument, drop_shadow_size) {
        return Some(sized(size.to_string(), drop_shadows(&size)));
    }
    if let Some(value) = arbitrary_or_variable(argument)
        .filter(|_| argument.starts_with('['))
        .filter(|value| {
            value_type(None, value) != Some(ValueType::Color)
                || split_top_level(value, ' ').len() > 1
        })
    {
        return Some(sized(value, String::from("var(--tw-drop-shadow-size)")));
    }
    let color = shadow_color(argument)?;
    Some(vec![
        decl(
            "--tw-drop-shadow-color",
            format!("color-mix(in oklab, {color} var(--tw-drop-shadow-alpha), transparent)"),
        ),
        decl("--tw-drop-shadow", "var(--tw-drop-shadow-size)"),
    ])
}

/// The color of a ring when no `ring-<color>` sets one
const RING_COLOR: &str = "currentcolor";

/// The declarations of `ring`, `inset-ring` and `ring-offset` utilities: a
/// width, which draws the ring, or a color, which the ring reads
fn ring_utility(name: &str) -> Option<Vec<Declaration>> {
    if name == "ring-inset" {
        return Some(vec![decl("--tw-ring-inset", "inset")]);
    }
    let (root, argument) = if let Some(argument) = name.strip_prefix("ring-offset") {
        ("ring-offset", argument)
    } else if let Some(argument) = name.strip_prefix("inset-ring") {
        ("inset-ring", argument)
    } else {
        ("ring", name.strip_prefix("ring")?)
    };
    let argument = if argument.is_empty() {
        None
    } else {
        Some(
            argument
                .strip_prefix('-')
                .filter(|argument| !argument.is_empty())?,
        )
    };
    let width = argument.and_then(|argument| {
        if is_positive_integer(argument) {
            Some(format!("{argument}px"))
        } else {
            arbitrary_or_variable(argument)
                .filter(|_| argument.starts_with('['))
                .filter(|value| value_type(None, value) == Some(ValueType::Length))
        }
    });
    let default_width = argument.is_none().then(|| String::from("1px"));
    match (width.or(default_width), root) {
        (Some(width), "ring") => Some(vec![
            decl(
                "--tw-ring-shadow",
                format!(
                    "var(--tw-ring-inset, ) 0 0 0 calc({width} + var(--tw-ring-offset-width)) var(--tw-ring-color, {RING_COLOR})"
                ),
            ),
            decl("box-shadow", BOX_SHADOW),
        ]),
        (Some(width), "inset-ring") => Some(vec![
            decl(
                "--tw-inset-ring-shadow",
                format!("inset 0 0 0 {width} var(--tw-inset-ring-color, currentcolor)"),
            ),
            decl("box-shadow", BOX_SHADOW),
        ]),
        (Some(width), _) if argument.is_some() => Some(vec![
            decl("--tw-ring-offset-width", width),
            decl(
                "--tw-ring-offset-shadow",
                "var(--tw-ring-inset, ) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)",
            ),
        ]),
        (Some(_), _) => None,
        (None, _) => {
            let color = shadow_color(argument?)?;
            let property = match root {
                "ring" => "--tw-ring-color",
                "inset-ring" => "--tw-inset-ring-color",
                _ => "--tw-ring-offset-color",
            };
            Some(vec![decl(property, color)])
        }
    }
}
/// The directions of `bg-linear-*`
fn linear_direction(name: &str) -> Option<&'static str> {
    Some(match name {
        "to-t" => "to top",
        "to-tr" => "to top right",
        "to-r" => "to right",
        "to-br" => "to bottom right",
        "to-b" => "to bottom",
        "to-bl" => "to bottom left",
        "to-l" => "to left",
        "to-tl" => "to top left",
        _ => return None,
    })
}

/// The color space the gradient `modifier` (`/oklch`, `/longer`) interpolates in
fn interpolation(modifier: Option<&str>) -> Option<String> {
    let Some(modifier) = modifier else {
        return Some(String::from("in oklab"));
    };
    if let Some(value) = modifier
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        return Some(decode_arbitrary_value(value));
    }
    Some(match modifier {
        "longer" | "shorter" | "increasing" | "decreasing" => {
            format!("in oklch {modifier} hue")
        }
        "oklab" | "oklch" | "srgb" | "hsl" => format!("in {modifier}"),
        _ => return None,
    })
}

/// A gradient: its position and the image drawn from the stops
fn gradient(position: String, function: &str, fallback: Option<&str>) -> Vec<Declaration> {
    let stops = fallback.map_or_else(
        || String::from("var(--tw-gradient-stops)"),
        |fallback| format!("var(--tw-gradient-stops,{fallback})"),
    );
    vec![
        decl("--tw-gradient-position", position),
        decl("background-image", format!("{function}-gradient({stops})")),
    ]
}

/// `bg-linear-*`, `bg-conic-*` and `bg-radial-*`
fn gradient_utility(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    let rest = name.strip_prefix("bg-")?;
    let parts = split_top_level(rest, '/');
    let (&root, modifier) = (parts.first()?, parts.get(1).copied());
    if parts.len() > 2 {
        return None;
    }
    let (kind, argument) = match root.split_once('-') {
        Some((kind, argument)) => (kind, Some(argument)),
        None => (root, None),
    };
    if !matches!(kind, "linear" | "conic" | "radial") {
        return None;
    }
    if let Some(value) = argument.and_then(arbitrary_or_variable) {
        if modifier.is_some() || negative {
            return None;
        }
        return Some(gradient(value.clone(), kind, Some(&value)));
    }
    let method = interpolation(modifier)?;
    let angle = |degrees: &str| {
        is_positive_integer(degrees).then(|| {
            if negative {
                format!("calc({degrees}deg * -1)")
            } else {
                format!("{degrees}deg")
            }
        })
    };
    let position = match (kind, argument) {
        ("linear", Some(argument)) => match linear_direction(argument) {
            Some(direction) if !negative => format!("{direction} {method}"),
            Some(_) => return None,
            None => format!("{} {method}", angle(argument)?),
        },
        ("conic", Some(argument)) => format!("from {} {method}", angle(argument)?),
        ("conic" | "radial", None) if !negative => method,
        _ => return None,
    };
    Some(gradient(position, kind, None))
}

/// The stops a color stop utility sets
const STOPS: &str = "var(--tw-gradient-via-stops, var(--tw-gradient-position), var(--tw-gradient-from) var(--tw-gradient-from-position), var(--tw-gradient-to) var(--tw-gradient-to-position))";

/// The stops once `via` is set
const VIA_STOPS: &str = "var(--tw-gradient-position), var(--tw-gradient-from) var(--tw-gradient-from-position), var(--tw-gradient-via) var(--tw-gradient-via-position), var(--tw-gradient-to) var(--tw-gradient-to-position)";

/// A color or a position of a gradient stop: `from-red-500`, `via-10%`,
/// `to-[#fff]`
fn stop_utility(name: &str) -> Option<Vec<Declaration>> {
    if name == "via-none" {
        return Some(vec![decl("--tw-gradient-via-stops", "initial")]);
    }
    let (stop, rest) = name.split_once('-')?;
    if !matches!(stop, "from" | "via" | "to") {
        return None;
    }
    let position = |value: String| {
        let property = match stop {
            "from" => "--tw-gradient-from-position",
            "via" => "--tw-gradient-via-position",
            _ => "--tw-gradient-to-position",
        };
        Some(vec![decl(property, value)])
    };
    if let Some(percent) = rest.strip_suffix('%') {
        return if is_positive_integer(percent) {
            position(rest.to_string())
        } else {
            None
        };
    }
    if let Some(value) = rest
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        let value = decode_arbitrary_value(value);
        if value_type(None, &value) == Some(ValueType::Length) {
            return position(value);
        }
    }
    let color = match arbitrary_color(rest) {
        Argument::Named => color_value(rest)?,
        Argument::Color(color) => color,
        Argument::Invalid => return None,
    };
    Some(match stop {
        "from" => vec![
            decl("--tw-gradient-from", color),
            decl("--tw-gradient-stops", STOPS),
        ],
        "via" => vec![
            decl("--tw-gradient-via", color),
            decl("--tw-gradient-via-stops", VIA_STOPS),
            decl("--tw-gradient-stops", "var(--tw-gradient-via-stops)"),
        ],
        _ => vec![
            decl("--tw-gradient-to", color),
            decl("--tw-gradient-stops", STOPS),
        ],
    })
}

/// The declarations of the utilities of this module, `None` for any other
pub fn effect_utility(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    if let Some(declarations) = filter_utility(name, negative) {
        return Some(declarations);
    }
    if let Some(declarations) = gradient_utility(name, negative) {
        return Some(declarations);
    }
    if negative {
        return None;
    }
    shadow_utility(name)
        .or_else(|| ring_utility(name))
        .or_else(|| drop_shadow_utility(name))
        .or_else(|| stop_utility(name))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use css::theme_tokens::set_theme_token_levels;
    use rstest::rstest;
    use serial_test::serial;

    use super::*;
    use crate::tailwind::{TailwindClass, declarations_of, parse_class};

    fn written(class: &str) -> Vec<(String, String)> {
        declarations_of(class).unwrap_or_else(|| panic!("{class} stays as written"))
    }

    fn expect(class: &str, expected: &[(&str, &str)]) {
        let expected: Vec<(String, String)> = expected
            .iter()
            .map(|(property, value)| (property.to_string(), value.to_string()))
            .collect();
        assert_eq!(written(class), expected, "{class}");
    }

    fn filter(variable: &str, value: &str) -> Vec<(String, String)> {
        vec![
            (variable.to_string(), value.to_string()),
            ("filter".to_string(), FILTER.to_string()),
        ]
    }

    fn backdrop(variable: &str, value: &str) -> Vec<(String, String)> {
        vec![
            (variable.to_string(), value.to_string()),
            (
                "-webkit-backdrop-filter".to_string(),
                BACKDROP_FILTER.to_string(),
            ),
            ("backdrop-filter".to_string(), BACKDROP_FILTER.to_string()),
        ]
    }

    #[rstest]
    #[case("blur", "blur(8px)")]
    #[case("blur-xs", "blur(4px)")]
    #[case("blur-sm", "blur(8px)")]
    #[case("blur-md", "blur(12px)")]
    #[case("blur-lg", "blur(16px)")]
    #[case("blur-xl", "blur(24px)")]
    #[case("blur-2xl", "blur(40px)")]
    #[case("blur-3xl", "blur(64px)")]
    #[case("blur-none", " ")]
    #[case("blur-[2px]", "blur(2px)")]
    #[case("blur-(--radius)", "blur(var(--radius))")]
    fn blur_composes_through_its_variable(#[case] class: &str, #[case] value: &str) {
        assert_eq!(written(class), filter("--tw-blur", value));
    }

    #[rstest]
    #[case("brightness-50", "--tw-brightness", "brightness(50%)")]
    #[case("brightness-125", "--tw-brightness", "brightness(125%)")]
    #[case("brightness-[1.75]", "--tw-brightness", "brightness(1.75)")]
    #[case("contrast-200", "--tw-contrast", "contrast(200%)")]
    #[case("grayscale", "--tw-grayscale", "grayscale(100%)")]
    #[case("grayscale-0", "--tw-grayscale", "grayscale(0%)")]
    #[case("grayscale-25", "--tw-grayscale", "grayscale(25%)")]
    #[case("hue-rotate-90", "--tw-hue-rotate", "hue-rotate(90deg)")]
    #[case("-hue-rotate-90", "--tw-hue-rotate", "hue-rotate(calc(90deg * -1))")]
    #[case(
        "-hue-rotate-[15deg]",
        "--tw-hue-rotate",
        "hue-rotate(calc(15deg * -1))"
    )]
    #[case("invert", "--tw-invert", "invert(100%)")]
    #[case("invert-0", "--tw-invert", "invert(0%)")]
    #[case("saturate-150", "--tw-saturate", "saturate(150%)")]
    #[case("sepia", "--tw-sepia", "sepia(100%)")]
    #[case("sepia-50", "--tw-sepia", "sepia(50%)")]
    fn filter_functions_set_their_own_variable(
        #[case] class: &str,
        #[case] variable: &str,
        #[case] value: &str,
    ) {
        assert_eq!(written(class), filter(variable, value));
    }

    #[rstest]
    #[case("backdrop-blur", "--tw-backdrop-blur", "blur(8px)")]
    #[case("backdrop-blur-xl", "--tw-backdrop-blur", "blur(24px)")]
    #[case("backdrop-blur-none", "--tw-backdrop-blur", " ")]
    #[case("backdrop-blur-[2px]", "--tw-backdrop-blur", "blur(2px)")]
    #[case(
        "backdrop-brightness-75",
        "--tw-backdrop-brightness",
        "brightness(75%)"
    )]
    #[case("backdrop-contrast-50", "--tw-backdrop-contrast", "contrast(50%)")]
    #[case("backdrop-grayscale", "--tw-backdrop-grayscale", "grayscale(100%)")]
    #[case(
        "backdrop-hue-rotate-15",
        "--tw-backdrop-hue-rotate",
        "hue-rotate(15deg)"
    )]
    #[case(
        "-backdrop-hue-rotate-15",
        "--tw-backdrop-hue-rotate",
        "hue-rotate(calc(15deg * -1))"
    )]
    #[case("backdrop-invert-25", "--tw-backdrop-invert", "invert(25%)")]
    #[case("backdrop-opacity-50", "--tw-backdrop-opacity", "opacity(50%)")]
    #[case("backdrop-opacity-12.5", "--tw-backdrop-opacity", "opacity(12.5%)")]
    #[case("backdrop-saturate-200", "--tw-backdrop-saturate", "saturate(200%)")]
    #[case("backdrop-sepia", "--tw-backdrop-sepia", "sepia(100%)")]
    fn backdrop_filter_functions_compose_too(
        #[case] class: &str,
        #[case] variable: &str,
        #[case] value: &str,
    ) {
        assert_eq!(written(class), backdrop(variable, value));
    }

    #[test]
    fn filter_and_backdrop_filter_read_every_function() {
        expect("filter", &[("filter", FILTER)]);
        expect("filter-none", &[("filter", "none")]);
        expect("filter-[url(#a)]", &[("filter", "url(#a)")]);
        expect("filter-(--f)", &[("filter", "var(--f)")]);
        expect(
            "backdrop-filter",
            &[
                ("-webkit-backdrop-filter", BACKDROP_FILTER),
                ("backdrop-filter", BACKDROP_FILTER),
            ],
        );
        expect(
            "backdrop-filter-none",
            &[
                ("-webkit-backdrop-filter", "none"),
                ("backdrop-filter", "none"),
            ],
        );
        expect(
            "backdrop-filter-[blur(2px)]",
            &[
                ("-webkit-backdrop-filter", "blur(2px)"),
                ("backdrop-filter", "blur(2px)"),
            ],
        );
        assert_eq!(FILTER.matches("var(--tw-").count(), 9);
        assert_eq!(BACKDROP_FILTER.matches("var(--tw-backdrop-").count(), 9);
    }

    #[test]
    fn two_filters_add_up_instead_of_overwriting() {
        let blur = written("blur-sm");
        let grayscale = written("grayscale");
        let shared = blur.last().unwrap();
        assert_eq!(shared, grayscale.last().unwrap());
        assert_ne!(blur[0].0, grayscale[0].0);
        let rules = [
            parse_class("blur-sm").unwrap(),
            parse_class("grayscale").unwrap(),
        ]
        .iter()
        .flat_map(TailwindClass::rules)
        .collect::<Vec<_>>();
        assert!(
            rules
                .iter()
                .any(|rule| rule == "@property --tw-blur{syntax:\"*\";inherits:false}")
        );
        assert!(
            rules
                .iter()
                .any(|rule| rule == "@property --tw-grayscale{syntax:\"*\";inherits:false}")
        );
    }

    #[rstest]
    #[case("blur-huge")]
    #[case("blur-")]
    #[case("blur-[]")]
    #[case("blur-(radius)")]
    #[case("-blur-sm")]
    #[case("brightness")]
    #[case("brightness-")]
    #[case("brightness-05")]
    #[case("brightness-1.5")]
    #[case("brightness-sm")]
    #[case("-brightness-50")]
    #[case("hue-rotate")]
    #[case("opacity-blur")]
    #[case("backdrop-opacity")]
    #[case("backdrop-opacity-13.3")]
    #[case("backdrop-blur-none/50")]
    #[case("filter-blur")]
    #[case("filter-[]")]
    #[case("-filter")]
    #[case("-filter-none")]
    #[case("backdrop-")]
    #[case("blurry")]
    fn unknown_filters_stay_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    #[rstest]
    #[case("shadow-2xs", "0 1px var(--tw-shadow-color, rgb(0 0 0 / 0.05))")]
    #[case("shadow-xs", "0 1px 2px 0 var(--tw-shadow-color, rgb(0 0 0 / 0.05))")]
    #[case(
        "shadow",
        "0 1px 3px 0 var(--tw-shadow-color, rgb(0 0 0 / 0.1)), 0 1px 2px -1px var(--tw-shadow-color, rgb(0 0 0 / 0.1))"
    )]
    #[case(
        "shadow-sm",
        "0 1px 3px 0 var(--tw-shadow-color, rgb(0 0 0 / 0.1)), 0 1px 2px -1px var(--tw-shadow-color, rgb(0 0 0 / 0.1))"
    )]
    #[case(
        "shadow-md",
        "0 4px 6px -1px var(--tw-shadow-color, rgb(0 0 0 / 0.1)), 0 2px 4px -2px var(--tw-shadow-color, rgb(0 0 0 / 0.1))"
    )]
    #[case(
        "shadow-lg",
        "0 10px 15px -3px var(--tw-shadow-color, rgb(0 0 0 / 0.1)), 0 4px 6px -4px var(--tw-shadow-color, rgb(0 0 0 / 0.1))"
    )]
    #[case(
        "shadow-xl",
        "0 20px 25px -5px var(--tw-shadow-color, rgb(0 0 0 / 0.1)), 0 8px 10px -6px var(--tw-shadow-color, rgb(0 0 0 / 0.1))"
    )]
    #[case(
        "shadow-2xl",
        "0 25px 50px -12px var(--tw-shadow-color, rgb(0 0 0 / 0.25))"
    )]
    #[case("shadow-none", "0 0 #0000")]
    fn shadow_sizes_follow_the_default_theme(#[case] class: &str, #[case] shadow: &str) {
        expect(
            class,
            &[("--tw-shadow", shadow), ("box-shadow", BOX_SHADOW)],
        );
    }

    #[test]
    #[serial]
    fn shadow_tokens_are_the_css_variables_of_the_theme() {
        set_theme_token_levels(
            BTreeMap::new(),
            BTreeMap::from([(String::from("card"), vec![0, 3])]),
        );
        expect(
            "shadow-card",
            &[("--tw-shadow", "var(--card)"), ("box-shadow", BOX_SHADOW)],
        );
        set_theme_token_levels(BTreeMap::new(), BTreeMap::new());
        assert_eq!(declarations_of("shadow-card"), None);
    }

    #[rstest]
    #[case(
        "shadow-red-500",
        "color-mix(in oklab, oklch(63.7% 0.237 25.331) var(--tw-shadow-alpha), transparent)"
    )]
    #[case(
        "shadow-red-500/50",
        "color-mix(in oklab, color-mix(in oklab, oklch(63.7% 0.237 25.331) 50%, transparent) var(--tw-shadow-alpha), transparent)"
    )]
    #[case(
        "shadow-[#fff]",
        "color-mix(in oklab, #fff var(--tw-shadow-alpha), transparent)"
    )]
    #[case(
        "shadow-[#fff]/50",
        "color-mix(in oklab, color-mix(in oklab, #fff 50%, transparent) var(--tw-shadow-alpha), transparent)"
    )]
    #[case(
        "shadow-current",
        "color-mix(in oklab, currentcolor var(--tw-shadow-alpha), transparent)"
    )]
    fn shadow_colors_set_the_color_the_shadows_read(#[case] class: &str, #[case] color: &str) {
        expect(class, &[("--tw-shadow-color", color)]);
    }

    #[test]
    fn shadow_inherit_and_unknown_shadows() {
        expect("shadow-inherit", &[("--tw-shadow-color", "inherit")]);
        for class in [
            "shadow-",
            "shadow-huge",
            "shadow-red-500/13.3",
            "shadow-[1px_1px_red]/50",
            "shadow-[#fff]/1/2",
        ] {
            assert_eq!(declarations_of(class), None, "{class}");
        }
    }

    #[test]
    fn shadow_recoloring_keeps_what_has_no_color() {
        assert_eq!(recolored("0 0", "--tw-shadow-color"), "0 0");
        assert_eq!(recolored("inset 0 1px", "--tw-shadow-color"), "inset 0 1px");
        assert_eq!(
            recolored("0 0 #0000", "--tw-shadow-color"),
            "0 0 var(--tw-shadow-color, #0000)"
        );
        assert_eq!(
            recolored("1px 1px red", "--tw-shadow-color"),
            "1px 1px var(--tw-shadow-color, red)"
        );
        assert_eq!(
            recolored("0 1px rgb(0 0 0 / 0.05), 0 0 #0000", "--tw-shadow-color"),
            "0 1px var(--tw-shadow-color, rgb(0 0 0 / 0.05)), 0 0 var(--tw-shadow-color, #0000)"
        );
    }

    #[rstest]
    #[case("bg-linear-to-t", "to top in oklab")]
    #[case("bg-linear-to-tr", "to top right in oklab")]
    #[case("bg-linear-to-r", "to right in oklab")]
    #[case("bg-linear-to-br", "to bottom right in oklab")]
    #[case("bg-linear-to-b", "to bottom in oklab")]
    #[case("bg-linear-to-bl", "to bottom left in oklab")]
    #[case("bg-linear-to-l", "to left in oklab")]
    #[case("bg-linear-to-tl", "to top left in oklab")]
    #[case("bg-linear-45", "45deg in oklab")]
    #[case("-bg-linear-45", "calc(45deg * -1) in oklab")]
    #[case("bg-linear-0", "0deg in oklab")]
    #[case("bg-linear-to-r/srgb", "to right in srgb")]
    #[case("bg-linear-to-r/hsl", "to right in hsl")]
    #[case("bg-linear-to-r/oklch", "to right in oklch")]
    #[case("bg-linear-to-r/oklab", "to right in oklab")]
    #[case("bg-linear-to-r/longer", "to right in oklch longer hue")]
    #[case("bg-linear-to-r/shorter", "to right in oklch shorter hue")]
    #[case("bg-linear-to-r/increasing", "to right in oklch increasing hue")]
    #[case("bg-linear-to-r/decreasing", "to right in oklch decreasing hue")]
    #[case("bg-linear-90/[in_hsl_longer_hue]", "90deg in hsl longer hue")]
    fn linear_gradients_use_the_syntax_of_v4(#[case] class: &str, #[case] position: &str) {
        expect(
            class,
            &[
                ("--tw-gradient-position", position),
                (
                    "background-image",
                    "linear-gradient(var(--tw-gradient-stops))",
                ),
            ],
        );
    }

    #[test]
    fn conic_and_radial_gradients() {
        for (class, position, image) in [
            (
                "bg-conic",
                "in oklab",
                "conic-gradient(var(--tw-gradient-stops))",
            ),
            (
                "bg-conic/srgb",
                "in srgb",
                "conic-gradient(var(--tw-gradient-stops))",
            ),
            (
                "bg-conic-180",
                "from 180deg in oklab",
                "conic-gradient(var(--tw-gradient-stops))",
            ),
            (
                "-bg-conic-180",
                "from calc(180deg * -1) in oklab",
                "conic-gradient(var(--tw-gradient-stops))",
            ),
            (
                "bg-conic-90/longer",
                "from 90deg in oklch longer hue",
                "conic-gradient(var(--tw-gradient-stops))",
            ),
            (
                "bg-radial",
                "in oklab",
                "radial-gradient(var(--tw-gradient-stops))",
            ),
            (
                "bg-radial/oklch",
                "in oklch",
                "radial-gradient(var(--tw-gradient-stops))",
            ),
        ] {
            expect(
                class,
                &[
                    ("--tw-gradient-position", position),
                    ("background-image", image),
                ],
            );
        }
    }

    #[test]
    fn arbitrary_gradients_carry_their_own_stops() {
        expect(
            "bg-linear-[25deg,red_5%,yellow_60%,lime_90%,teal]",
            &[
                (
                    "--tw-gradient-position",
                    "25deg,red 5%,yellow 60%,lime 90%,teal",
                ),
                (
                    "background-image",
                    "linear-gradient(var(--tw-gradient-stops,25deg,red 5%,yellow 60%,lime 90%,teal))",
                ),
            ],
        );
        expect(
            "bg-radial-[at_50%_75%,#fff,#000]",
            &[
                ("--tw-gradient-position", "at 50% 75%,#fff,#000"),
                (
                    "background-image",
                    "radial-gradient(var(--tw-gradient-stops,at 50% 75%,#fff,#000))",
                ),
            ],
        );
        expect(
            "bg-conic-(--stops)",
            &[
                ("--tw-gradient-position", "var(--stops)"),
                (
                    "background-image",
                    "conic-gradient(var(--tw-gradient-stops,var(--stops)))",
                ),
            ],
        );
    }

    #[rstest]
    #[case("bg-gradient-to-r")]
    #[case("bg-gradient-to-br")]
    #[case("bg-linear")]
    #[case("bg-linear-")]
    #[case("bg-linear-sideways")]
    #[case("bg-linear-4.5")]
    #[case("bg-linear-045")]
    #[case("-bg-linear-to-r")]
    #[case("-bg-linear")]
    #[case("-bg-radial")]
    #[case("-bg-conic")]
    #[case("-bg-linear-[10deg,red,blue]")]
    #[case("bg-linear-[10deg,red,blue]/oklch")]
    #[case("bg-linear-to-r/neon")]
    #[case("bg-linear-to-r/oklch/srgb")]
    #[case("bg-radial-45")]
    #[case("bg-conic-x")]
    #[case("bg-sweep-45")]
    fn gradients_v4_does_not_define_stay_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    const STOPS: &str = "var(--tw-gradient-via-stops, var(--tw-gradient-position), var(--tw-gradient-from) var(--tw-gradient-from-position), var(--tw-gradient-to) var(--tw-gradient-to-position))";
    const VIA_STOPS: &str = "var(--tw-gradient-position), var(--tw-gradient-from) var(--tw-gradient-from-position), var(--tw-gradient-via) var(--tw-gradient-via-position), var(--tw-gradient-to) var(--tw-gradient-to-position)";

    #[test]
    fn gradient_stops_compose_through_variables() {
        expect(
            "from-red-500",
            &[
                ("--tw-gradient-from", "oklch(63.7% 0.237 25.331)"),
                ("--tw-gradient-stops", STOPS),
            ],
        );
        expect(
            "via-white",
            &[
                ("--tw-gradient-via", "#fff"),
                ("--tw-gradient-via-stops", VIA_STOPS),
                ("--tw-gradient-stops", "var(--tw-gradient-via-stops)"),
            ],
        );
        expect(
            "to-blue-500/50",
            &[
                (
                    "--tw-gradient-to",
                    "color-mix(in oklab, oklch(62.3% 0.214 259.815) 50%, transparent)",
                ),
                ("--tw-gradient-stops", STOPS),
            ],
        );
        expect(
            "from-[#f00]",
            &[
                ("--tw-gradient-from", "#f00"),
                ("--tw-gradient-stops", STOPS),
            ],
        );
        expect(
            "from-[#f00]/50",
            &[
                (
                    "--tw-gradient-from",
                    "color-mix(in oklab, #f00 50%, transparent)",
                ),
                ("--tw-gradient-stops", STOPS),
            ],
        );
        expect("via-none", &[("--tw-gradient-via-stops", "initial")]);
    }

    #[test]
    fn gradient_stop_positions() {
        expect("from-10%", &[("--tw-gradient-from-position", "10%")]);
        expect("via-30%", &[("--tw-gradient-via-position", "30%")]);
        expect("to-90%", &[("--tw-gradient-to-position", "90%")]);
        expect("from-[12px]", &[("--tw-gradient-from-position", "12px")]);
        expect("to-[40%]", &[("--tw-gradient-to-position", "40%")]);
    }

    #[rstest]
    #[case("from-")]
    #[case("from-1.5%")]
    #[case("from-05%")]
    #[case("from-%")]
    #[case("from-[12px]/50")]
    #[case("from-[url(a.png)]")]
    #[case("from-[]")]
    #[case("from-nothing")]
    #[case("from-red-500/")]
    #[case("-from-red-500")]
    #[case("via-none/50")]
    #[case("up-red-500")]
    fn unknown_gradient_stops_stay_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    #[test]
    fn gradient_rules_register_the_stop_variables() {
        let rules = parse_class("from-red-500").unwrap().rules();
        for name in [
            "--tw-gradient-position",
            "--tw-gradient-from",
            "--tw-gradient-to",
            "--tw-gradient-stops",
            "--tw-gradient-via-stops",
            "--tw-gradient-from-position",
            "--tw-gradient-to-position",
        ] {
            assert!(
                rules
                    .iter()
                    .any(|rule| rule.starts_with(&format!("@property {name}{{"))),
                "{name}"
            );
        }
        let rules = parse_class("via-30%").unwrap().rules();
        assert_eq!(
            rules,
            vec![
                "@property --tw-gradient-via-position{syntax:\"<length-percentage>\";inherits:false;initial-value:50%}"
            ]
        );
    }

    #[test]
    fn modifiers_and_variants_apply_to_composed_utilities() {
        let class = parse_class("hover:blur-sm").unwrap();
        assert_eq!(class.declarations, written_pairs("blur-sm"));
        assert_eq!(class.conditions.len(), 1);
        assert_eq!(effect_utility("shadow-md", true), None);
        assert_eq!(effect_utility("nothing", false), None);
    }

    fn written_pairs(
        class: &str,
    ) -> Vec<(
        std::borrow::Cow<'static, str>,
        std::borrow::Cow<'static, str>,
    )> {
        written(class)
            .into_iter()
            .map(|(property, value)| (property.into(), value.into()))
            .collect()
    }

    #[rstest]
    #[case("ring", "1px")]
    #[case("ring-0", "0px")]
    #[case("ring-2", "2px")]
    #[case("ring-8", "8px")]
    #[case("ring-[3px]", "3px")]
    fn ring_widths_draw_the_ring_composed_with_the_shadows(
        #[case] class: &str,
        #[case] width: &str,
    ) {
        expect(
            class,
            &[
                (
                    "--tw-ring-shadow",
                    &format!(
                        "var(--tw-ring-inset, ) 0 0 0 calc({width} + var(--tw-ring-offset-width)) var(--tw-ring-color, currentcolor)"
                    ),
                ),
                ("box-shadow", BOX_SHADOW),
            ],
        );
    }

    #[test]
    fn ring_colors_inset_rings_and_offsets() {
        expect("ring-inset", &[("--tw-ring-inset", "inset")]);
        expect(
            "ring-red-500",
            &[("--tw-ring-color", "oklch(63.7% 0.237 25.331)")],
        );
        expect(
            "ring-red-500/50",
            &[(
                "--tw-ring-color",
                "color-mix(in oklab, oklch(63.7% 0.237 25.331) 50%, transparent)",
            )],
        );
        expect("ring-[#f00]", &[("--tw-ring-color", "#f00")]);
        expect(
            "inset-ring",
            &[
                (
                    "--tw-inset-ring-shadow",
                    "inset 0 0 0 1px var(--tw-inset-ring-color, currentcolor)",
                ),
                ("box-shadow", BOX_SHADOW),
            ],
        );
        expect(
            "inset-ring-4",
            &[
                (
                    "--tw-inset-ring-shadow",
                    "inset 0 0 0 4px var(--tw-inset-ring-color, currentcolor)",
                ),
                ("box-shadow", BOX_SHADOW),
            ],
        );
        expect("inset-ring-white", &[("--tw-inset-ring-color", "#fff")]);
        expect(
            "ring-offset-2",
            &[
                ("--tw-ring-offset-width", "2px"),
                (
                    "--tw-ring-offset-shadow",
                    "var(--tw-ring-inset, ) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)",
                ),
            ],
        );
        expect(
            "ring-offset-[3px]",
            &[
                ("--tw-ring-offset-width", "3px"),
                (
                    "--tw-ring-offset-shadow",
                    "var(--tw-ring-inset, ) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)",
                ),
            ],
        );
        expect("ring-offset-black", &[("--tw-ring-offset-color", "#000")]);
    }

    #[rstest]
    #[case("ring-")]
    #[case("ringo")]
    #[case("ring-1.5")]
    #[case("ring-02")]
    #[case("ring-nothing")]
    #[case("ring-[red]/50/1")]
    #[case("ring-offset")]
    #[case("ring-offset-")]
    #[case("ring-offsetx")]
    #[case("ring-offset-nothing")]
    #[case("inset-ring-nothing")]
    #[case("-ring-2")]
    fn unknown_rings_stay_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    #[rstest]
    #[case("drop-shadow-xs", "0 1px 1px", "rgb(0 0 0 / 0.05)")]
    #[case("drop-shadow-sm", "0 1px 2px", "rgb(0 0 0 / 0.15)")]
    #[case("drop-shadow-md", "0 3px 3px", "rgb(0 0 0 / 0.12)")]
    #[case("drop-shadow-lg", "0 4px 4px", "rgb(0 0 0 / 0.15)")]
    #[case("drop-shadow-xl", "0 9px 7px", "rgb(0 0 0 / 0.1)")]
    #[case("drop-shadow-2xl", "0 25px 25px", "rgb(0 0 0 / 0.15)")]
    fn drop_shadow_sizes_compose_into_filter(
        #[case] class: &str,
        #[case] offsets: &str,
        #[case] color: &str,
    ) {
        expect(
            class,
            &[
                (
                    "--tw-drop-shadow-size",
                    &format!("drop-shadow({offsets} var(--tw-drop-shadow-color, {color}))"),
                ),
                (
                    "--tw-drop-shadow",
                    &format!("drop-shadow({offsets} {color})"),
                ),
                ("filter", FILTER),
            ],
        );
    }
    #[test]
    fn drop_shadow_without_a_size_none_colors_and_arbitrary_values() {
        expect(
            "drop-shadow",
            &[
                (
                    "--tw-drop-shadow-size",
                    "drop-shadow(0 1px 2px var(--tw-drop-shadow-color, rgb(0 0 0 / 0.1))) drop-shadow(0 1px 1px var(--tw-drop-shadow-color, rgb(0 0 0 / 0.06)))",
                ),
                (
                    "--tw-drop-shadow",
                    "drop-shadow(0 1px 2px rgb(0 0 0 / 0.1)) drop-shadow(0 1px 1px rgb(0 0 0 / 0.06))",
                ),
                ("filter", FILTER),
            ],
        );
        expect(
            "drop-shadow-none",
            &[("--tw-drop-shadow", " "), ("filter", FILTER)],
        );
        expect(
            "drop-shadow-inherit",
            &[
                ("--tw-drop-shadow-color", "inherit"),
                ("--tw-drop-shadow", "var(--tw-drop-shadow-size)"),
            ],
        );
        expect(
            "drop-shadow-red-500",
            &[
                (
                    "--tw-drop-shadow-color",
                    "color-mix(in oklab, oklch(63.7% 0.237 25.331) var(--tw-drop-shadow-alpha), transparent)",
                ),
                ("--tw-drop-shadow", "var(--tw-drop-shadow-size)"),
            ],
        );
        expect(
            "drop-shadow-[#0003]",
            &[
                (
                    "--tw-drop-shadow-color",
                    "color-mix(in oklab, #0003 var(--tw-drop-shadow-alpha), transparent)",
                ),
                ("--tw-drop-shadow", "var(--tw-drop-shadow-size)"),
            ],
        );
        expect(
            "drop-shadow-[0_0_4px_#000]",
            &[
                (
                    "--tw-drop-shadow-size",
                    "drop-shadow(0 0 4px var(--tw-drop-shadow-color, #000))",
                ),
                ("--tw-drop-shadow", "var(--tw-drop-shadow-size)"),
                ("filter", FILTER),
            ],
        );
        for class in [
            "drop-shadow-",
            "drop-shadow-huge",
            "drop-shadow-md/50",
            "drop-shadow-[]",
        ] {
            assert_eq!(declarations_of(class), None, "{class}");
        }
    }
}
