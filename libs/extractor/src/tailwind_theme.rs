//! The theme values Tailwind utilities read: what the project's Tailwind CSS
//! `@theme` defines, else Tailwind CSS v4's defaults

use std::borrow::Cow;

use css::tailwind_definitions::{namespace_is_reset, theme_value, theme_variable};

/// The value of `--<namespace>-<key>` (`--<namespace>` for an empty key) that
/// the project defines, else the default, which a project can empty
pub fn themed(
    namespace: &str,
    key: &str,
    default: impl FnOnce(&str) -> Option<&'static str>,
) -> Option<Cow<'static, str>> {
    let custom = if key.is_empty() {
        theme_variable(namespace)
    } else {
        theme_value(namespace, key)
    };
    if let Some(value) = custom {
        return Some(Cow::Owned(value));
    }
    if namespace_is_reset(namespace) {
        return None;
    }
    default(key).map(Cow::Borrowed)
}

/// What a class prefix reads from the theme but this build cannot compile
/// from `@theme`, as `(prefix, namespace)`
static UNSUPPORTED: &[(&str, &str)] = &[
    ("text-", "text"),
    ("font-", "font"),
    ("font-", "font-weight"),
    ("leading-", "leading"),
    ("tracking-", "tracking"),
    ("max-w-", "container"),
    ("min-w-", "container"),
    ("w-", "container"),
    ("inset-shadow-", "inset-shadow"),
    ("perspective-", "perspective"),
    ("aspect-", "aspect"),
];

/// Whether `name` is a utility whose theme values the project changes in a way
/// this build does not read, which must stay as written
pub fn reads_unsupported_theme(name: &str) -> bool {
    UNSUPPORTED.iter().any(|(prefix, namespace)| {
        name.strip_prefix(prefix).is_some_and(|key| {
            let key = key.split('/').next().unwrap_or(key);
            theme_value(namespace, key).is_some() || namespace_is_reset(namespace)
        })
    })
}

/// `n` times `base`, for a number `n` Tailwind scales the spacing by
pub fn scaled(key: &str, base: &str) -> Option<String> {
    let number: f64 = key.parse().ok()?;
    if number < 0.0 || number % 0.25 != 0.0 || number.to_string() != key {
        return None;
    }
    let split = base.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (amount, unit) = base.split_at(split);
    let amount: f64 = amount.parse().ok()?;
    if unit.is_empty() || !unit.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }
    let product = (number * amount * 1e6).round() / 1e6;
    Some(if product == 0.0 {
        String::from("0px")
    } else {
        format!("{product}{unit}")
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use css::tailwind_definitions::set_tailwind_css;
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn themed_values_come_from_the_project_first() {
        let default = |key: &str| (key == "sm").then_some("4px");
        assert_eq!(themed("blur", "sm", default).as_deref(), Some("4px"));
        assert_eq!(themed("blur", "xl", default), None);
        set_tailwind_css("@theme { --blur-sm: 1px; --blur: 9px; --blur-huge: 99px; }");
        assert_eq!(themed("blur", "sm", default).as_deref(), Some("1px"));
        assert_eq!(themed("blur", "huge", default).as_deref(), Some("99px"));
        assert_eq!(themed("blur", "", default).as_deref(), Some("9px"));
        set_tailwind_css("@theme { --blur-*: initial; }");
        assert_eq!(themed("blur", "sm", default), None);
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn utilities_that_read_namespaces_the_build_does_not_read() {
        assert!(!reads_unsupported_theme("text-lg"));
        set_tailwind_css(
            "@theme { --text-lg: 2rem; --font-weight-bold: 800; --container-sm: 20rem; }",
        );
        assert!(reads_unsupported_theme("text-lg"));
        assert!(reads_unsupported_theme("text-lg/6"));
        assert!(!reads_unsupported_theme("text-sm"));
        assert!(reads_unsupported_theme("font-bold"));
        assert!(!reads_unsupported_theme("font-thin"));
        assert!(reads_unsupported_theme("max-w-sm"));
        assert!(!reads_unsupported_theme("p-4"));
        set_tailwind_css("@theme { --*: initial; }");
        assert!(reads_unsupported_theme("leading-tight"));
        assert!(reads_unsupported_theme("w-4"));
        set_tailwind_css("");
        assert!(!reads_unsupported_theme("leading-tight"));
    }

    #[test]
    fn spacing_scales_by_the_number() {
        assert_eq!(scaled("13", "0.25rem").as_deref(), Some("3.25rem"));
        assert_eq!(scaled("0.5", "4px").as_deref(), Some("2px"));
        assert_eq!(scaled("3", "0.3rem").as_deref(), Some("0.9rem"));
        assert_eq!(scaled("0", "0.25rem").as_deref(), Some("0px"));
        assert_eq!(scaled("1.3", "0.25rem"), None);
        assert_eq!(scaled("01", "0.25rem"), None);
        assert_eq!(scaled("-1", "0.25rem"), None);
        assert_eq!(scaled("x", "0.25rem"), None);
        assert_eq!(scaled("4", "2"), None);
        assert_eq!(scaled("4", "calc(1px)"), None);
        assert_eq!(scaled("4", "var(--x)"), None);
        assert_eq!(scaled("4", "rem"), None);
    }

    fn written(class: &str) -> Vec<(String, String)> {
        crate::tailwind::declarations_of(class)
            .unwrap_or_else(|| panic!("{class} stays as written"))
    }

    fn pairs(declarations: &[(&str, &str)]) -> Vec<(String, String)> {
        declarations
            .iter()
            .map(|(property, value)| (property.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    #[serial]
    fn theme_colors_extend_and_reset_the_palette() {
        set_tailwind_css(
            "@theme { --color-brand: #0af; --color-mint: var(--color-emerald-500); --color-alias: var(--color-brand); }",
        );
        assert_eq!(written("bg-brand"), pairs(&[("background-color", "#0af")]));
        assert_eq!(written("bg-alias"), pairs(&[("background-color", "#0af")]));
        assert_eq!(
            written("bg-mint"),
            pairs(&[("background-color", "oklch(69.6% 0.17 162.48)")])
        );
        assert_eq!(
            written("text-brand/50"),
            pairs(&[("color", "color-mix(in oklab, #0af 50%, transparent)")])
        );
        assert_eq!(
            written("bg-red-500"),
            pairs(&[("background-color", "oklch(63.7% 0.237 25.331)")])
        );
        set_tailwind_css("@theme { --color-*: initial; --color-brand: #0af; }");
        assert_eq!(written("bg-brand"), pairs(&[("background-color", "#0af")]));
        assert_eq!(crate::tailwind::declarations_of("bg-white"), None);
        assert_eq!(crate::tailwind::declarations_of("bg-red-500"), None);
        assert_eq!(
            written("bg-current"),
            pairs(&[("background-color", "currentcolor")])
        );
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn theme_spacing_scales_every_step() {
        assert_eq!(written("p-13"), pairs(&[("padding", "3.25rem")]));
        assert_eq!(written("p-4"), pairs(&[("padding", "1rem")]));
        set_tailwind_css("@theme { --spacing: 0.3rem; --spacing-gutter: 3rem; }");
        assert_eq!(written("p-4"), pairs(&[("padding", "1.2rem")]));
        assert_eq!(written("mx-0"), pairs(&[("margin-inline", "0px")]));
        assert_eq!(written("p-px"), pairs(&[("padding", "1px")]));
        assert_eq!(written("gap-gutter"), pairs(&[("gap", "3rem")]));
        assert_eq!(written("-mt-2"), pairs(&[("margin-top", "-0.6rem")]));
        set_tailwind_css("@theme { --spacing: calc(1px + 1px); }");
        assert_eq!(written("p-4"), pairs(&[("padding", "1rem")]));
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn theme_shadows_blurs_radii_and_easings() {
        set_tailwind_css(
            "@theme { --shadow-soft: 0 2px 8px #0003, inset 0 0 1px red; --blur-huge: 100px; --blur: 5px; --radius-pill: 40px; --radius: 3px; --ease-snappy: cubic-bezier(0.2, 0, 0, 1); }",
        );
        assert_eq!(
            written("shadow-soft"),
            pairs(&[
                (
                    "--tw-shadow",
                    "0 2px 8px var(--tw-shadow-color, #0003), inset 0 0 1px var(--tw-shadow-color, red)"
                ),
                (
                    "box-shadow",
                    "var(--tw-inset-shadow,0 0 #0000), var(--tw-inset-ring-shadow,0 0 #0000), var(--tw-ring-offset-shadow,0 0 #0000), var(--tw-ring-shadow,0 0 #0000), var(--tw-shadow)"
                ),
            ])
        );
        assert_eq!(
            written("blur-huge")[0],
            ("--tw-blur".to_string(), "blur(100px)".to_string())
        );
        assert_eq!(written("blur")[0].1, "blur(5px)");
        assert_eq!(written("blur-sm")[0].1, "blur(8px)");
        assert_eq!(written("rounded-pill"), pairs(&[("border-radius", "40px")]));
        assert_eq!(written("rounded"), pairs(&[("border-radius", "3px")]));
        assert_eq!(written("rounded-lg"), pairs(&[("border-radius", "0.5rem")]));
        assert_eq!(
            written("ease-snappy"),
            pairs(&[
                ("--tw-ease", "cubic-bezier(0.2, 0, 0, 1)"),
                ("transition-timing-function", "cubic-bezier(0.2, 0, 0, 1)"),
            ])
        );
        set_tailwind_css(
            "@theme { --shadow-*: initial; --blur-*: initial; --radius-*: initial; --ease-*: initial; --animate-*: initial; }",
        );
        for class in [
            "shadow-md",
            "shadow",
            "blur-sm",
            "blur",
            "rounded-lg",
            "rounded",
            "ease-in",
            "animate-spin",
        ] {
            assert_eq!(crate::tailwind::declarations_of(class), None, "{class}");
        }
        assert_eq!(
            written("ease-linear"),
            pairs(&[
                ("--tw-ease", "linear"),
                ("transition-timing-function", "linear")
            ])
        );
        assert_eq!(written("animate-none"), pairs(&[("animation", "none")]));
        assert_eq!(
            written("rounded-full"),
            pairs(&[("border-radius", "calc(infinity * 1px)")])
        );
        assert_eq!(written("rounded-none"), pairs(&[("border-radius", "0")]));
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn theme_animations_bring_their_keyframes() {
        set_tailwind_css(
            "@theme { --animate-wiggle: wiggle 1s ease-in-out infinite; @keyframes wiggle { 0%, 100% { transform: rotate(-3deg) } 50% { transform: rotate(3deg) } } --default-transition-duration: 300ms; --default-transition-timing-function: linear; }",
        );
        assert_eq!(
            written("animate-wiggle"),
            pairs(&[("animation", "wiggle 1s ease-in-out infinite")])
        );
        let rules = crate::tailwind::parse_class("animate-wiggle")
            .unwrap()
            .rules();
        assert_eq!(
            rules,
            vec!["@keyframes wiggle{0%,100%{transform:rotate(-3deg)}50%{transform:rotate(3deg)}}"]
        );
        assert_eq!(
            written("transition-opacity")[1..],
            pairs(&[
                ("transition-timing-function", "var(--tw-ease, linear)"),
                ("transition-duration", "var(--tw-duration, 300ms)"),
            ])
        );
        set_tailwind_css("");
        assert_eq!(crate::tailwind::declarations_of("animate-wiggle"), None);
    }

    #[test]
    #[serial]
    fn theme_breakpoints_and_custom_variants() {
        set_tailwind_css(
            "@theme { --breakpoint-3xl: 120rem; }
@custom-variant theme-midnight (&:where([data-theme=\"midnight\"] *));
@custom-variant dark (&:where(.dark, .dark *));
@custom-variant pointer-hover { @media (hover: hover) { @slot; } }",
        );
        let conditions = |class: &str| {
            crate::tailwind::parse_class(class)
                .unwrap()
                .conditions
                .iter()
                .map(|condition| condition.as_ref().map(ToString::to_string))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            conditions("3xl:p-4"),
            vec![Some(String::from("@media(min-width:120rem)"))]
        );
        assert_eq!(
            conditions("theme-midnight:p-4"),
            vec![Some(String::from("&:where([data-theme=\"midnight\"] *)"))]
        );
        assert_eq!(
            conditions("dark:p-4"),
            vec![Some(String::from("&:where(.dark, .dark *)"))]
        );
        assert_eq!(
            conditions("pointer-hover:p-4"),
            vec![Some(String::from("@media(hover:hover)"))]
        );
        assert_eq!(crate::tailwind::parse_class("midnight:p-4"), None);
        set_tailwind_css("");
        assert_eq!(crate::tailwind::parse_class("3xl:p-4"), None);
        assert_eq!(
            conditions("dark:p-4"),
            vec![Some(String::from(":root[data-theme=dark] &"))]
        );
    }

    #[test]
    #[serial]
    fn custom_utilities_are_their_declarations() {
        set_tailwind_css(
            "@utility content-auto { content-visibility: auto; contain-intrinsic-size: auto 500px; } @utility tab-* { tab-size: --value(integer); }",
        );
        assert_eq!(
            written("content-auto"),
            pairs(&[
                ("content-visibility", "auto"),
                ("contain-intrinsic-size", "auto 500px"),
            ])
        );
        assert_eq!(
            written("content-auto!"),
            pairs(&[
                ("content-visibility", "auto !important"),
                ("contain-intrinsic-size", "auto 500px !important"),
            ])
        );
        assert_eq!(crate::tailwind::declarations_of("-content-auto"), None);
        assert_eq!(crate::tailwind::declarations_of("tab-4"), None);
        set_tailwind_css("");
        assert_eq!(crate::tailwind::declarations_of("content-auto"), None);
    }

    #[test]
    #[serial]
    fn utilities_of_changed_namespaces_stay_as_written() {
        assert!(crate::tailwind::declarations_of("text-lg").is_some());
        set_tailwind_css("@theme { --text-lg: 2rem; --font-weight-bold: 800; }");
        assert_eq!(crate::tailwind::declarations_of("text-lg"), None);
        assert_eq!(crate::tailwind::declarations_of("font-bold"), None);
        assert!(crate::tailwind::declarations_of("text-sm").is_some());
        assert!(crate::tailwind::declarations_of("font-thin").is_some());
        set_tailwind_css("");
    }
}
