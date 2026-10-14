//! `space-*` and `divide-*`: utilities that style the children of the element,
//! all but the last, through the selector Tailwind CSS v4 writes for them

use crate::tailwind::{
    Declaration, arbitrary_or_variable, decl, is_positive_integer, spacing_scale,
};
use crate::tailwind_color::color_value;

/// The children `space-*` and `divide-*` style, which `:where` leaves to any
/// utility of the child itself
pub const CHILDREN: &str = ":where(& > :not(:last-child))";

/// The value of the spacing `argument`: a step of the scale, or an arbitrary
/// value, negated by a leading `-`
pub(crate) fn spacing(argument: &str, negative: bool) -> Option<String> {
    if let Some(value) = spacing_scale(argument) {
        return Some(if negative && value != "0px" {
            format!("-{value}")
        } else {
            value.to_string()
        });
    }
    let value = arbitrary_or_variable(argument)?;
    Some(if negative {
        format!("calc({value} * -1)")
    } else {
        value
    })
}
/// The margins of `space-x-*` and `space-y-*`
fn space_utility(axis: char, argument: &str, negative: bool) -> Option<Vec<Declaration>> {
    let reverse = format!("--tw-space-{axis}-reverse");
    if argument == "reverse" {
        return (!negative).then(|| vec![decl(reverse, "1")]);
    }
    let value = spacing(argument, negative)?;
    let (start, end) = if axis == 'x' {
        ("margin-inline-start", "margin-inline-end")
    } else {
        ("margin-block-start", "margin-block-end")
    };
    if matches!(value.as_str(), "0px" | "0") {
        return Some(vec![decl(start, "0"), decl(end, "0")]);
    }
    Some(vec![
        decl(start, format!("calc({value} * var({reverse}))")),
        decl(end, format!("calc({value} * calc(1 - var({reverse})))")),
    ])
}

/// The border width of `divide-x-*` and `divide-y-*`: a number of pixels,
/// or an arbitrary value
fn divide_width(argument: &str) -> Option<String> {
    if is_positive_integer(argument) {
        return Some(format!("{argument}px"));
    }
    arbitrary_or_variable(argument)
}

/// The borders of `divide-x-*` and `divide-y-*`
fn divide_width_utility(axis: char, argument: &str) -> Option<Vec<Declaration>> {
    let reverse = format!("--tw-divide-{axis}-reverse");
    if argument == "-reverse" {
        return Some(vec![decl(reverse, "1")]);
    }
    let width = if argument.is_empty() {
        String::from("1px")
    } else {
        divide_width(argument.strip_prefix('-')?)?
    };
    let start = format!("calc({width} * var({reverse}))");
    let end = format!("calc({width} * calc(1 - var({reverse})))");
    Some(if axis == 'x' {
        vec![
            decl("border-inline-style", "var(--tw-border-style)"),
            decl("border-inline-start-width", start),
            decl("border-inline-end-width", end),
        ]
    } else {
        vec![
            decl("border-bottom-style", "var(--tw-border-style)"),
            decl("border-top-style", "var(--tw-border-style)"),
            decl("border-top-width", start),
            decl("border-bottom-width", end),
        ]
    })
}
/// The declarations of `space-*` and `divide-*`, which style the children
pub fn children_utility(name: &str) -> Option<Vec<Declaration>> {
    let (negative, name) = name
        .strip_prefix('-')
        .map_or((false, name), |name| (true, name));
    if let Some(argument) = name.strip_prefix("space-x-") {
        return space_utility('x', argument, negative);
    }
    if let Some(argument) = name.strip_prefix("space-y-") {
        return space_utility('y', argument, negative);
    }
    if negative {
        return None;
    }
    let argument = name.strip_prefix("divide-")?;
    if let Some(declarations) = argument
        .strip_prefix('x')
        .and_then(|argument| divide_width_utility('x', argument))
        .or_else(|| {
            argument
                .strip_prefix('y')
                .and_then(|argument| divide_width_utility('y', argument))
        })
    {
        return Some(declarations);
    }
    if matches!(argument, "solid" | "dashed" | "dotted" | "double" | "none") {
        return Some(vec![
            decl("--tw-border-style", argument.to_string()),
            decl("border-style", argument.to_string()),
        ]);
    }
    color_value(argument).map(|color| vec![decl("border-color", color)])
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::tailwind::{declarations_of, parse_class};

    fn expect(class: &str, expected: &[(&str, &str)]) {
        let expected: Vec<(String, String)> = expected
            .iter()
            .map(|(property, value)| (property.to_string(), value.to_string()))
            .collect();
        assert_eq!(declarations_of(class), Some(expected), "{class}");
    }

    #[rstest]
    #[case("space-x-0", &[("margin-inline-start", "0"), ("margin-inline-end", "0")])]
    #[case("space-x-px", &[
        ("margin-inline-start", "calc(1px * var(--tw-space-x-reverse))"),
        ("margin-inline-end", "calc(1px * calc(1 - var(--tw-space-x-reverse)))"),
    ])]
    #[case("space-x-4", &[
        ("margin-inline-start", "calc(1rem * var(--tw-space-x-reverse))"),
        ("margin-inline-end", "calc(1rem * calc(1 - var(--tw-space-x-reverse)))"),
    ])]
    #[case("-space-x-4", &[
        ("margin-inline-start", "calc(-1rem * var(--tw-space-x-reverse))"),
        ("margin-inline-end", "calc(-1rem * calc(1 - var(--tw-space-x-reverse)))"),
    ])]
    #[case("-space-x-px", &[
        ("margin-inline-start", "calc(-1px * var(--tw-space-x-reverse))"),
        ("margin-inline-end", "calc(-1px * calc(1 - var(--tw-space-x-reverse)))"),
    ])]
    #[case("-space-y-0", &[("margin-block-start", "0"), ("margin-block-end", "0")])]
    #[case("space-y-2.5", &[
        ("margin-block-start", "calc(0.625rem * var(--tw-space-y-reverse))"),
        ("margin-block-end", "calc(0.625rem * calc(1 - var(--tw-space-y-reverse)))"),
    ])]
    #[case("space-y-[3px]", &[
        ("margin-block-start", "calc(3px * var(--tw-space-y-reverse))"),
        ("margin-block-end", "calc(3px * calc(1 - var(--tw-space-y-reverse)))"),
    ])]
    #[case("space-x-(--gap)", &[
        ("margin-inline-start", "calc(var(--gap) * var(--tw-space-x-reverse))"),
        ("margin-inline-end", "calc(var(--gap) * calc(1 - var(--tw-space-x-reverse)))"),
    ])]
    #[case("-space-x-[3px]", &[
        ("margin-inline-start", "calc(calc(3px * -1) * var(--tw-space-x-reverse))"),
        ("margin-inline-end", "calc(calc(3px * -1) * calc(1 - var(--tw-space-x-reverse)))"),
    ])]
    #[case("-space-y-(--gap)", &[
        ("margin-block-start", "calc(calc(var(--gap) * -1) * var(--tw-space-y-reverse))"),
        ("margin-block-end", "calc(calc(var(--gap) * -1) * calc(1 - var(--tw-space-y-reverse)))"),
    ])]
    #[case("space-x-reverse", &[("--tw-space-x-reverse", "1")])]
    #[case("space-y-reverse", &[("--tw-space-y-reverse", "1")])]
    fn space_adds_margins_between_the_children(
        #[case] class: &str,
        #[case] expected: &[(&str, &str)],
    ) {
        expect(class, expected);
    }

    #[rstest]
    #[case("divide-x", "x", "1px")]
    #[case("divide-x-0", "x", "0px")]
    #[case("divide-x-2", "x", "2px")]
    #[case("divide-x-8", "x", "8px")]
    #[case("divide-x-[3px]", "x", "3px")]
    #[case("divide-x-(--w)", "x", "var(--w)")]
    #[case("divide-y", "y", "1px")]
    #[case("divide-y-4", "y", "4px")]
    fn divide_borders_the_children(#[case] class: &str, #[case] axis: &str, #[case] width: &str) {
        if axis == "x" {
            expect(
                class,
                &[
                    ("border-inline-style", "var(--tw-border-style)"),
                    (
                        "border-inline-start-width",
                        &format!("calc({width} * var(--tw-divide-x-reverse))"),
                    ),
                    (
                        "border-inline-end-width",
                        &format!("calc({width} * calc(1 - var(--tw-divide-x-reverse)))"),
                    ),
                ],
            );
        } else {
            expect(
                class,
                &[
                    ("border-bottom-style", "var(--tw-border-style)"),
                    ("border-top-style", "var(--tw-border-style)"),
                    (
                        "border-top-width",
                        &format!("calc({width} * var(--tw-divide-y-reverse))"),
                    ),
                    (
                        "border-bottom-width",
                        &format!("calc({width} * calc(1 - var(--tw-divide-y-reverse)))"),
                    ),
                ],
            );
        }
    }

    #[test]
    fn divide_reverse_style_and_color() {
        expect("divide-x-reverse", &[("--tw-divide-x-reverse", "1")]);
        expect("divide-y-reverse", &[("--tw-divide-y-reverse", "1")]);
        for style in ["solid", "dashed", "dotted", "double", "none"] {
            expect(
                &format!("divide-{style}"),
                &[("--tw-border-style", style), ("border-style", style)],
            );
        }
        expect(
            "divide-red-500",
            &[("border-color", "oklch(63.7% 0.237 25.331)")],
        );
        expect(
            "divide-yellow-500/50",
            &[(
                "border-color",
                "color-mix(in oklab, oklch(79.5% 0.184 86.047) 50%, transparent)",
            )],
        );
    }

    #[rstest]
    #[case("divide-xyz")]
    #[case("divide-x-")]
    #[case("divide-x-1.5")]
    #[case("divide-x-05")]
    #[case("divide-x-reverse-2")]
    #[case("divide-y-abc")]
    #[case("divide-purple")]
    #[case("divide-")]
    #[case("-divide-x-2")]
    #[case("-divide-red-500")]
    #[case("space-x-")]
    #[case("space-x-reverse-1")]
    #[case("space-x-1.3")]
    #[case("space-x-huge")]
    #[case("-space-x-reverse")]
    #[case("space-z-4")]
    fn unknown_children_utilities_stay_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    #[test]
    fn children_utilities_apply_under_variants_to_the_children() {
        let class = parse_class("space-x-4").unwrap();
        assert_eq!(class.conditions.len(), 1);
        assert_eq!(
            class.conditions[0]
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some(CHILDREN)
        );
        let class = parse_class("md:space-x-4").unwrap();
        assert_eq!(class.level, 2);
        assert_eq!(
            class.conditions[0]
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some(CHILDREN)
        );
        let class = parse_class("focus:divide-x-2").unwrap();
        assert_eq!(
            class.conditions[0]
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some(":where(&:focus > :not(:last-child))")
        );
        let class = parse_class("space-x-4!").unwrap();
        assert!(
            class
                .declarations
                .iter()
                .all(|(_, value)| value.ends_with(" !important"))
        );
    }

    #[test]
    fn children_rules_register_their_variables() {
        assert_eq!(
            parse_class("space-x-4").unwrap().rules(),
            vec!["@property --tw-space-x-reverse{syntax:\"*\";inherits:false;initial-value:0}"]
        );
        assert_eq!(
            parse_class("divide-x-2").unwrap().rules(),
            vec![
                "@property --tw-divide-x-reverse{syntax:\"*\";inherits:false;initial-value:0}",
                "@property --tw-border-style{syntax:\"*\";inherits:false;initial-value:solid}",
            ]
        );
    }
}
