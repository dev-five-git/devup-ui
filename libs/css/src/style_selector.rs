use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};

use crate::{constant::SELECTOR_ORDER_MAP, selector_separator::SelectorSeparator, to_kebab_case};

#[derive(Debug, PartialEq, Clone, Hash, Eq, Serialize, Deserialize)]
pub enum StyleSelector {
    Media {
        query: String,
        selector: Option<String>,
    },
    Selector(String),
    // selector, file
    Global(String, String),
}

impl PartialOrd for StyleSelector {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for StyleSelector {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (
                StyleSelector::Media {
                    query: a,
                    selector: aa,
                },
                StyleSelector::Media {
                    query: b,
                    selector: bb,
                },
            ) => {
                let c = a.cmp(b);
                if c == Ordering::Equal { aa.cmp(bb) } else { c }
            }
            (StyleSelector::Selector(a), StyleSelector::Selector(b)) => {
                get_selector_order(a).cmp(&get_selector_order(b))
            }
            (
                StyleSelector::Media {
                    selector: _,
                    query: _,
                },
                StyleSelector::Selector(_),
            ) => Ordering::Greater,
            (
                StyleSelector::Selector(_),
                StyleSelector::Media {
                    selector: _,
                    query: _,
                },
            ) => Ordering::Less,
            (StyleSelector::Global(a, _), StyleSelector::Global(b, _)) => {
                if a == b {
                    return Ordering::Equal;
                }
                match (a.contains(":"), b.contains(":")) {
                    (true, true) => {
                        let a_order = format!(":{}", a.split(":").nth(1).unwrap());
                        let b_order = format!(":{}", b.split(":").nth(1).unwrap());
                        let mut a_order_value = 0;
                        let mut b_order_value = 0;
                        for (order, order_value) in SELECTOR_ORDER_MAP.iter() {
                            if a_order.contains(order) {
                                a_order_value = *order_value;
                            }
                            if b_order.contains(order) {
                                b_order_value = *order_value;
                            }
                        }
                        if a_order_value == b_order_value {
                            a.cmp(b)
                        } else {
                            a_order_value.cmp(&b_order_value)
                        }
                    }
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => a.cmp(b),
                }
            }
            (StyleSelector::Global(_, _), _) => Ordering::Less,
            (_, StyleSelector::Global(_, _)) => Ordering::Greater,
        }
    }
}

impl From<&str> for StyleSelector {
    fn from(value: &str) -> Self {
        let value = value
            .trim()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace(", ", ",");
        if value.contains("&") {
            StyleSelector::Selector(value.to_string())
        } else if let Some(s) = value.strip_prefix("group") {
            let post = to_kebab_case(s);
            StyleSelector::Selector(format!(
                "{}{}{} &",
                "*[role=group]",
                SelectorSeparator::from(post.as_str()),
                post
            ))
        } else if let Some(s) = value.strip_prefix("theme") {
            // first character should lower case
            StyleSelector::Selector(format!(
                ":root[data-theme={}{}] &",
                s.chars().next().unwrap().to_ascii_lowercase(),
                &s[1..]
            ))
        } else if value == "print" {
            StyleSelector::Media {
                query: "print".to_string(),
                selector: None,
            }
        } else {
            let post = to_kebab_case(&value);

            StyleSelector::Selector(format!(
                "&{}{}",
                SelectorSeparator::from(post.as_str()),
                post
            ))
        }
    }
}

impl From<[&str; 2]> for StyleSelector {
    fn from(value: [&str; 2]) -> Self {
        let post = if value[1].contains("&:") {
            to_kebab_case(value[1].split(":").last().unwrap())
        } else {
            to_kebab_case(value[1])
        };
        StyleSelector::Selector(format!(
            "{}{}{}",
            StyleSelector::from(value[0]),
            SelectorSeparator::from(post.as_str()),
            post
        ))
    }
}
impl From<(&StyleSelector, &str)> for StyleSelector {
    fn from(value: (&StyleSelector, &str)) -> Self {
        if let StyleSelector::Global(_, file) = value.0 {
            let post = to_kebab_case(value.1);
            StyleSelector::Global(
                format!(
                    "{}{}{}",
                    value.0,
                    SelectorSeparator::from(post.as_str()),
                    post
                ),
                file.clone(),
            )
        } else {
            StyleSelector::from([&value.0.to_string(), value.1])
        }
    }
}

impl Display for StyleSelector {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StyleSelector::Selector(value) => value.to_string(),
                StyleSelector::Media { query, selector } => {
                    if let Some(selector) = selector {
                        format!("@{query} {selector}")
                    } else {
                        format!("@{query}")
                    }
                }
                StyleSelector::Global(value, _) => value.to_string(),
            }
        )
    }
}

fn get_selector_order(selector: &str) -> u8 {
    // & count
    let t = if selector.chars().filter(|c| c == &'&').count() == 1 {
        selector
            .split('&')
            .next_back()
            .map(|a| a.to_string())
            .unwrap_or(selector.to_string())
    } else {
        selector.to_string()
    };

    *SELECTOR_ORDER_MAP
        .get(&t)
        .unwrap_or(if t.starts_with("&") { &0 } else { &99 })
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case("hover", StyleSelector::Selector("&:hover".to_string()))]
    #[case("focusVisible", StyleSelector::Selector("&:focus-visible".to_string()))]
    #[case("groupHover", StyleSelector::Selector("*[role=group]:hover &".to_string()))]
    #[case("groupFocusVisible", StyleSelector::Selector("*[role=group]:focus-visible &".to_string()))]
    #[case("group1", StyleSelector::Selector("*[role=group]:1 &".to_string()))]
    #[case(["themeDark", "placeholder"], StyleSelector::Selector(":root[data-theme=dark] &::placeholder".to_string()))]
    #[case("themeLight", StyleSelector::Selector(":root[data-theme=light] &".to_string()))]
    #[case("*[aria=disabled='true'] &:hover", StyleSelector::Selector("*[aria=disabled='true'] &:hover".to_string()))]
    fn test_style_selector(
        #[case] input: impl Into<StyleSelector>,
        #[case] expected: StyleSelector,
    ) {
        assert_eq!(input.into(), expected);
    }

    #[rstest]
    #[case(StyleSelector::Selector("&:hover".to_string()), "&:hover")]
    #[case(StyleSelector::Media {
            query: "screen and (max-width: 600px)".to_string(),
            selector: None,
        },
        "@screen and (max-width: 600px)"
    )]
    #[case(StyleSelector::Global(":root[data-theme=dark]".to_string(), "file.rs".to_string()), ":root[data-theme=dark]")]
    fn test_style_selector_display(#[case] selector: StyleSelector, #[case] expected: &str) {
        let output = format!("{selector}");
        assert_eq!(output, expected);
    }

    #[rstest]
    #[case(
        StyleSelector::Media {
            query: "screen".to_string(),
            selector: None,
        },
        StyleSelector::Selector("&:hover".to_string()),
        std::cmp::Ordering::Greater
    )]
    #[case(
        StyleSelector::Selector("&:hover".to_string()),
        StyleSelector::Selector("&:focus-visible".to_string()),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Media {
            query: "a".to_string(),
            selector: None,
        },
        StyleSelector::Media {
            query: "b".to_string(),
            selector: None,
        },
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Global(":root[data-theme=dark]".to_string(), "file1.rs".to_string()),
        StyleSelector::Global(":root[data-theme=light]".to_string(), "file2.rs".to_string()),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::from(":root[data-theme=dark] &:hover"),
        StyleSelector::from(":root[data-theme=dark] &:focus-visible"),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Selector("&:hover".to_string()),
        StyleSelector::Media {
            query: "screen".to_string(),
            selector: None,
        },
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::from("&:hover"),
        StyleSelector::from("&:hover"),
        std::cmp::Ordering::Equal
    )]
    #[case(
        StyleSelector::Global(":root[data-theme=dark]".to_string(), "file1.rs".to_string()),
        StyleSelector::Global(":root[data-theme=dark]".to_string(), "file2.rs".to_string()),
        std::cmp::Ordering::Equal
    )]
    #[case(
        StyleSelector::Global("div".to_string(), "file1.rs".to_string()),
        StyleSelector::Global("div:hover".to_string(), "file2.rs".to_string()),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Global("div:hover".to_string(), "file2.rs".to_string()),
        StyleSelector::Global("div".to_string(), "file1.rs".to_string()),
        std::cmp::Ordering::Greater
    )]
    #[case(
        StyleSelector::Global("div:hover".to_string(), "file2.rs".to_string()),
        StyleSelector::Global("span:hover".to_string(), "file1.rs".to_string()),
        "div".cmp("span")
    )]
    #[case(
        StyleSelector::Global("div:hover".to_string(), "file2.rs".to_string()),
        StyleSelector::Global("span:focus".to_string(), "file1.rs".to_string()),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Global("div:".to_string(), "file2.rs".to_string()),
        StyleSelector::Global("span:".to_string(), "file1.rs".to_string()),
        "div".cmp("span")
    )]
    // global selector always less than selector
    #[case(
        StyleSelector::Global("div:".to_string(), "file2.rs".to_string()),
        StyleSelector::Selector("&:hover".to_string()),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Selector("&:hover".to_string()),
        StyleSelector::Global("div:".to_string(), "file2.rs".to_string()),
        std::cmp::Ordering::Greater
    )]
    fn test_style_selector_ord(
        #[case] a: StyleSelector,
        #[case] b: StyleSelector,
        #[case] expected: std::cmp::Ordering,
    ) {
        assert_eq!(a.cmp(&b), expected);
        assert_eq!(a.partial_cmp(&b), Some(expected));
    }

    #[rstest]
    #[case("&:hover", 0)]
    #[case("&:focus-visible", 1)]
    #[case("&:focus", 2)]
    #[case("&:active", 3)]
    #[case("&:selected", 4)]
    #[case("&:disabled", 5)]
    #[case("&:not-exist", 99)]
    #[case("&:not-exist, &:hover", 0)]
    #[case(":root[data-theme=dark] &:hover", 0)]
    #[case(":root[data-theme=dark] &:focus-visible", 1)]
    #[case(":root[data-theme=dark] &:focus", 2)]
    #[case(":root[data-theme=dark] &:active", 3)]
    #[case(":root[data-theme=dark] &:selected", 4)]
    #[case(":root[data-theme=dark] &:disabled", 5)]
    #[case(":root[data-theme=dark] &:not-exist", 99)]
    fn test_get_selector_order(#[case] selector: &str, #[case] expected: u8) {
        assert_eq!(get_selector_order(selector), expected);
    }
}
