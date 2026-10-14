//! Transitions and animations the way Tailwind CSS v4 writes them: the
//! duration and easing a `transition` uses are the custom properties
//! `duration-*` and `ease-*` set, and `animate-*` brings the `@keyframes` it
//! names

use std::borrow::Cow;

use css::tailwind_definitions::{theme_keyframes, theme_variable};

use crate::tailwind::{Declaration, arbitrary_or_variable, decl, is_positive_integer};
use crate::tailwind_theme::themed;

/// The easing a transition has when no `ease-*` sets one
fn timing() -> String {
    let default = theme_variable("default-transition-timing-function")
        .unwrap_or_else(|| String::from("cubic-bezier(0.4, 0, 0.2, 1)"));
    format!("var(--tw-ease, {default})")
}

/// The duration a transition has when no `duration-*` sets one
fn duration() -> String {
    let default =
        theme_variable("default-transition-duration").unwrap_or_else(|| String::from("150ms"));
    format!("var(--tw-duration, {default})")
}

/// What `transition` makes transition
const TRANSITION: &str = "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to, opacity, box-shadow, transform, translate, scale, rotate, filter, -webkit-backdrop-filter, backdrop-filter, display, content-visibility, overlay, pointer-events";

/// What `transition-colors` makes transition
const TRANSITION_COLORS: &str = "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to";

/// The `@keyframes` of the animations of the default theme
pub static KEYFRAMES: [(&str, &str); 4] = [
    ("spin", "@keyframes spin{to{transform:rotate(360deg)}}"),
    (
        "ping",
        "@keyframes ping{75%,100%{transform:scale(2);opacity:0}}",
    ),
    ("pulse", "@keyframes pulse{50%{opacity:0.5}}"),
    (
        "bounce",
        "@keyframes bounce{0%,100%{transform:translateY(-25%);animation-timing-function:cubic-bezier(0.8,0,1,1)}50%{transform:none;animation-timing-function:cubic-bezier(0,0,0.2,1)}}",
    ),
];

/// The value of `animate-<name>` for the animations of the default theme
fn animation(name: &str) -> Option<&'static str> {
    Some(match name {
        "spin" => "spin 1s linear infinite",
        "ping" => "ping 1s cubic-bezier(0, 0, 0.2, 1) infinite",
        "pulse" => "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "bounce" => "bounce 1s infinite",
        _ => return None,
    })
}

/// A transition of `property`, with the default easing and duration
fn transition(property: &str) -> Vec<Declaration> {
    vec![
        decl("transition-property", property.to_string()),
        decl("transition-timing-function", timing()),
        decl("transition-duration", duration()),
    ]
}

/// The theme easings of `ease-<name>`
fn easing(name: &str) -> Option<&'static str> {
    Some(match name {
        "in" => "cubic-bezier(0.4, 0, 1, 1)",
        "out" => "cubic-bezier(0, 0, 0.2, 1)",
        "in-out" => "cubic-bezier(0.4, 0, 0.2, 1)",
        _ => return None,
    })
}

/// The declarations of a `transition`, `duration`, `delay`, `ease` or
/// `animate` utility, `None` for any other
pub fn motion_utility(name: &str) -> Option<Vec<Declaration>> {
    if name == "transition" {
        return Some(transition(TRANSITION));
    }
    if let Some(argument) = name.strip_prefix("transition-") {
        return Some(match argument {
            "none" => vec![decl("transition-property", "none")],
            "all" => transition("all"),
            "colors" => transition(TRANSITION_COLORS),
            "opacity" => transition("opacity"),
            "shadow" => transition("box-shadow"),
            "transform" => transition("transform, translate, scale, rotate"),
            "discrete" => vec![decl("transition-behavior", "allow-discrete")],
            "normal" => vec![decl("transition-behavior", "normal")],
            _ => transition(&arbitrary_or_variable(argument)?),
        });
    }
    if let Some(argument) = name.strip_prefix("delay-") {
        let value = arbitrary_or_variable(argument).or_else(|| milliseconds(argument))?;
        return Some(vec![decl("transition-delay", value)]);
    }
    if let Some(argument) = name.strip_prefix("duration-") {
        if argument == "initial" {
            return Some(vec![decl("--tw-duration", "initial")]);
        }
        let value = arbitrary_or_variable(argument).or_else(|| milliseconds(argument))?;
        return Some(vec![
            decl("--tw-duration", value.clone()),
            decl("transition-duration", value),
        ]);
    }
    if let Some(argument) = name.strip_prefix("ease-") {
        if argument == "initial" {
            return Some(vec![decl("--tw-ease", "initial")]);
        }
        let value = if argument == "linear" {
            Some(String::from("linear"))
        } else {
            themed("ease", argument, easing)
                .map(Cow::into_owned)
                .or_else(|| arbitrary_or_variable(argument))
        }?;
        return Some(vec![
            decl("--tw-ease", value.clone()),
            decl("transition-timing-function", value),
        ]);
    }
    let argument = name.strip_prefix("animate-")?;
    let value = if argument == "none" {
        Some(String::from("none"))
    } else {
        themed("animate", argument, animation)
            .map(Cow::into_owned)
            .or_else(|| arbitrary_or_variable(argument))
    }?;
    Some(vec![decl("animation", value)])
}

/// `150` as `150ms`
fn milliseconds(value: &str) -> Option<String> {
    is_positive_integer(value).then(|| format!("{value}ms"))
}

/// The `@keyframes` rules the declarations name in an `animation`: the
/// project's, else those of the default theme
pub fn keyframes_of(declarations: &[Declaration]) -> impl Iterator<Item = Cow<'static, str>> + '_ {
    declarations
        .iter()
        .filter(|(property, _)| property == "animation")
        .flat_map(|(_, value)| value.split_whitespace())
        .filter_map(|word| {
            theme_keyframes(word).map(Cow::Owned).or_else(|| {
                KEYFRAMES
                    .iter()
                    .find(|(name, _)| *name == word)
                    .map(|&(_, rule)| Cow::Borrowed(rule))
            })
        })
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

    fn transition(property: &str) -> Vec<(String, String)> {
        vec![
            ("transition-property".to_string(), property.to_string()),
            ("transition-timing-function".to_string(), timing()),
            ("transition-duration".to_string(), duration()),
        ]
    }

    fn expect_owned(class: &str, expected: Vec<(String, String)>) {
        assert_eq!(declarations_of(class), Some(expected), "{class}");
    }

    #[test]
    fn transitions_read_the_easing_and_duration_utilities_set() {
        expect_owned("transition", transition(TRANSITION));
        expect_owned("transition-all", transition("all"));
        expect_owned("transition-colors", transition(TRANSITION_COLORS));
        expect_owned("transition-opacity", transition("opacity"));
        expect_owned("transition-shadow", transition("box-shadow"));
        expect_owned(
            "transition-transform",
            transition("transform, translate, scale, rotate"),
        );
        expect("transition-none", &[("transition-property", "none")]);
        expect_owned("transition-[height]", transition("height"));
        expect_owned("transition-[height,opacity]", transition("height,opacity"));
        expect_owned("transition-(--props)", transition("var(--props)"));
        expect(
            "transition-discrete",
            &[("transition-behavior", "allow-discrete")],
        );
        expect("transition-normal", &[("transition-behavior", "normal")]);
        assert_eq!(timing(), "var(--tw-ease, cubic-bezier(0.4, 0, 0.2, 1))");
        assert_eq!(duration(), "var(--tw-duration, 150ms)");
    }

    #[test]
    fn transitions_cover_what_v4_transitions() {
        for property in [
            "outline-color",
            "--tw-gradient-from",
            "translate",
            "scale",
            "rotate",
            "-webkit-backdrop-filter",
            "display",
            "content-visibility",
            "overlay",
            "pointer-events",
        ] {
            assert!(TRANSITION.contains(property), "{property}");
        }
        assert!(TRANSITION_COLORS.contains("outline-color"));
    }

    #[rstest]
    #[case("duration-0", "0ms")]
    #[case("duration-75", "75ms")]
    #[case("duration-150", "150ms")]
    #[case("duration-250", "250ms")]
    #[case("duration-1000", "1000ms")]
    #[case("duration-[2s]", "2s")]
    #[case("duration-(--speed)", "var(--speed)")]
    fn duration_sets_the_variable_a_transition_reads(#[case] class: &str, #[case] value: &str) {
        expect(
            class,
            &[("--tw-duration", value), ("transition-duration", value)],
        );
    }

    #[rstest]
    #[case("ease-in", "cubic-bezier(0.4, 0, 1, 1)")]
    #[case("ease-out", "cubic-bezier(0, 0, 0.2, 1)")]
    #[case("ease-in-out", "cubic-bezier(0.4, 0, 0.2, 1)")]
    #[case("ease-linear", "linear")]
    #[case("ease-[cubic-bezier(0.3,0,0,1)]", "cubic-bezier(0.3,0,0,1)")]
    #[case("ease-(--curve)", "var(--curve)")]
    fn ease_sets_the_variable_a_transition_reads(#[case] class: &str, #[case] value: &str) {
        expect(
            class,
            &[("--tw-ease", value), ("transition-timing-function", value)],
        );
    }

    #[test]
    fn initial_and_delay() {
        expect("duration-initial", &[("--tw-duration", "initial")]);
        expect("ease-initial", &[("--tw-ease", "initial")]);
        expect("delay-0", &[("transition-delay", "0ms")]);
        expect("delay-150", &[("transition-delay", "150ms")]);
        expect("delay-[3s]", &[("transition-delay", "3s")]);
        expect("delay-(--wait)", &[("transition-delay", "var(--wait)")]);
    }

    #[rstest]
    #[case("transition-sideways")]
    #[case("transition-[]")]
    #[case("transition-")]
    #[case("-transition")]
    #[case("duration")]
    #[case("duration-")]
    #[case("duration-1.5")]
    #[case("duration-050")]
    #[case("duration-fast")]
    #[case("-duration-150")]
    #[case("delay-1s")]
    #[case("delay-")]
    #[case("ease")]
    #[case("ease-bouncy")]
    #[case("animate")]
    #[case("animate-wobble")]
    #[case("animate-")]
    #[case("-animate-spin")]
    fn unknown_motion_stays_as_written(#[case] class: &str) {
        assert_eq!(declarations_of(class), None, "{class}");
    }

    #[rstest]
    #[case("animate-spin", "spin 1s linear infinite", "spin")]
    #[case("animate-ping", "ping 1s cubic-bezier(0, 0, 0.2, 1) infinite", "ping")]
    #[case(
        "animate-pulse",
        "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "pulse"
    )]
    #[case("animate-bounce", "bounce 1s infinite", "bounce")]
    fn animations_bring_their_keyframes(
        #[case] class: &str,
        #[case] animation: &str,
        #[case] keyframes: &str,
    ) {
        expect(class, &[("animation", animation)]);
        let rules = parse_class(class).unwrap().rules();
        assert_eq!(rules.len(), 1);
        assert!(
            rules[0].starts_with(&format!("@keyframes {keyframes}{{")),
            "{rules:?}"
        );
    }

    #[test]
    fn keyframes_match_the_default_theme_of_v4() {
        let rule = |name: &str| {
            KEYFRAMES
                .iter()
                .find(|(keyframes, _)| *keyframes == name)
                .unwrap()
                .1
        };
        assert_eq!(
            rule("spin"),
            "@keyframes spin{to{transform:rotate(360deg)}}"
        );
        assert_eq!(
            rule("ping"),
            "@keyframes ping{75%,100%{transform:scale(2);opacity:0}}"
        );
        assert_eq!(rule("pulse"), "@keyframes pulse{50%{opacity:0.5}}");
        assert_eq!(
            rule("bounce"),
            "@keyframes bounce{0%,100%{transform:translateY(-25%);animation-timing-function:cubic-bezier(0.8,0,1,1)}50%{transform:none;animation-timing-function:cubic-bezier(0,0,0.2,1)}}"
        );
    }

    #[test]
    fn arbitrary_animations_bring_the_keyframes_they_name() {
        expect("animate-none", &[("animation", "none")]);
        expect(
            "animate-[wiggle_1s_ease-in-out_infinite]",
            &[("animation", "wiggle 1s ease-in-out infinite")],
        );
        expect("animate-(--spin)", &[("animation", "var(--spin)")]);
        assert_eq!(
            parse_class("animate-[wiggle_1s_ease-in-out_infinite]")
                .unwrap()
                .rules(),
            Vec::<&str>::new()
        );
        assert_eq!(
            parse_class("animate-[spin_3s_linear_infinite]")
                .unwrap()
                .rules(),
            vec!["@keyframes spin{to{transform:rotate(360deg)}}"]
        );
        assert_eq!(
            parse_class("animate-none").unwrap().rules(),
            Vec::<&str>::new()
        );
    }
}
