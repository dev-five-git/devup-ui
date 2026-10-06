use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};

use crate::{
    at_rule::{MediaCombination, combine_media_queries, media_shorthand_query, normalize_query},
    constant::SELECTOR_ORDER,
    selector_separator::SelectorSeparator,
    to_kebab_case,
    utils::{collapse_whitespace, to_camel_case},
};

#[derive(
    Debug, PartialEq, PartialOrd, Ord, Clone, Copy, Hash, Eq, Serialize, Deserialize, Default,
)]
pub enum AtRuleKind {
    #[default]
    Media,
    Supports,
    Container,
    Layer,
}

impl Display for AtRuleKind {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            AtRuleKind::Media => write!(f, "media"),
            AtRuleKind::Supports => write!(f, "supports"),
            AtRuleKind::Container => write!(f, "container"),
            AtRuleKind::Layer => write!(f, "layer"),
        }
    }
}

impl From<&str> for AtRuleKind {
    fn from(value: &str) -> Self {
        match value {
            "media" => AtRuleKind::Media,
            "supports" => AtRuleKind::Supports,
            "container" => AtRuleKind::Container,
            "layer" => AtRuleKind::Layer,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Clone, Hash, Eq, Serialize, Deserialize)]
pub struct AtRule {
    pub kind: AtRuleKind,
    pub query: String,
}

#[derive(Debug, PartialEq, Clone, Hash, Eq, Serialize, Deserialize)]
pub enum StyleSelector {
    At {
        kind: AtRuleKind,
        query: String,
        selector: Option<String>,
        /// At-rules enclosing this one, outermost first. Only set when the
        /// rules cannot be folded into a single `@media` query.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        outer: Vec<AtRule>,
        /// Source file of a `globalCss` rule, so re-extracting that file drops it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file: Option<String>,
    },
    Selector(String),
    // selector, file
    Global(String, String),
}

/// Collapse an owned selector string's whitespace WITHOUT allocating when it is
/// already tight. The overwhelmingly common input (every selector produced by
/// `StyleSelector::from(&str)`, which already collapsed) hits
/// `collapse_whitespace`'s `Cow::Borrowed` fast-path, so the prior
/// `collapse_whitespace(&s).into_owned()` cloned an identical `String` for
/// nothing. Detecting the borrow lets us MOVE the already-owned `String`
/// through with zero allocation; only the rare interior-whitespace source
/// (e.g. a raw `StyleSelector::Selector(sel)` from `css_to_style`, which
/// `.trim()`s but does not collapse interior runs) pays for the owned rebuild.
/// Byte-identical to the previous behavior.
#[inline]
fn collapse_owned_selector(s: String) -> String {
    match collapse_whitespace(&s) {
        Cow::Borrowed(_) => s,
        Cow::Owned(collapsed) => collapsed,
    }
}

#[must_use]
pub fn optimize_selector(selector: StyleSelector) -> StyleSelector {
    match selector {
        StyleSelector::At {
            kind,
            query,
            selector,
            outer,
            file,
        } => StyleSelector::At {
            kind,
            query,
            selector: selector.map(collapse_owned_selector),
            outer,
            file,
        },
        StyleSelector::Selector(selector) => {
            StyleSelector::Selector(collapse_owned_selector(selector))
        }
        StyleSelector::Global(selector, file) => {
            StyleSelector::Global(collapse_owned_selector(selector), file)
        }
    }
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
                StyleSelector::At {
                    kind: ka,
                    query: a,
                    selector: aa,
                    outer: oa,
                    file: fa,
                },
                StyleSelector::At {
                    kind: kb,
                    query: b,
                    selector: bb,
                    outer: ob,
                    file: fb,
                },
            ) => (*ka as u8)
                .cmp(&(*kb as u8))
                .then_with(|| a.cmp(b))
                .then_with(|| aa.cmp(bb))
                .then_with(|| oa.cmp(ob))
                .then_with(|| fa.cmp(fb)),
            (StyleSelector::Selector(a), StyleSelector::Selector(b)) => {
                let order_cmp = get_selector_order(a).cmp(&get_selector_order(b));
                if order_cmp == Ordering::Equal {
                    a.cmp(b)
                } else {
                    order_cmp
                }
            }
            (StyleSelector::Global(a, _), StyleSelector::Global(b, _)) => {
                if a == b {
                    return Ordering::Equal;
                }
                match (a.contains(':'), b.contains(':')) {
                    (true, true) => {
                        // `contains(':')` is true here, so `find` always succeeds;
                        // the slice equals the former `format!(":{post}")` without allocating.
                        let a_order = a.find(':').map_or("", |i| &a[i..]);
                        let b_order = b.find(':').map_or("", |i| &b[i..]);
                        let a_order_value = global_selector_order(a_order);
                        let b_order_value = global_selector_order(b_order);
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
            (StyleSelector::At { .. }, StyleSelector::Selector(_))
            | (_, StyleSelector::Global(_, _)) => Ordering::Greater,
            (StyleSelector::Selector(_), StyleSelector::At { .. })
            | (StyleSelector::Global(_, _), _) => Ordering::Less,
        }
    }
}

impl From<&str> for StyleSelector {
    fn from(value: &str) -> Self {
        let value = collapse_whitespace(value);
        if value.contains('&') {
            StyleSelector::Selector(value.into_owned())
        } else if let Some(s) = value.strip_prefix("group-") {
            let post = to_kebab_case(s);
            let sep = SelectorSeparator::from(post.as_ref()).as_str();
            // Match both the legacy `role="group"` attribute (deprecated, removal
            // planned for v2) and the new `data-group` attribute with a single
            // `:is()` clause. `:is()` adopts its highest-specificity argument, so
            // the resulting specificity is identical to the prior `*[role=group]`
            // form. See CHANGELOG and docs/api/group-selector.
            StyleSelector::Selector(format!(":is([role=group],[data-group]){sep}{post} &"))
        } else if let Some(s) = value.strip_prefix("theme-") {
            // first character should lower case
            StyleSelector::Selector(format!(":root[data-theme={}] &", to_camel_case(s)))
        } else if let Some(query) = media_shorthand_query(&value) {
            StyleSelector::At {
                kind: AtRuleKind::Media,
                query: query.to_string(),
                selector: None,
                outer: vec![],
                file: None,
            }
        } else {
            let post = to_kebab_case(&value);

            StyleSelector::Selector(format!(
                "&{}{}",
                SelectorSeparator::from(post.as_ref()),
                post
            ))
        }
    }
}

impl From<[&str; 2]> for StyleSelector {
    fn from(value: [&str; 2]) -> Self {
        let post = if value[1].contains("&:") {
            to_kebab_case(value[1].rsplit_once(':').map_or(value[1], |(_, post)| post))
        } else {
            to_kebab_case(value[1])
        };
        StyleSelector::Selector(format!(
            "{}{}{}",
            StyleSelector::from(value[0]),
            SelectorSeparator::from(post.as_ref()),
            post
        ))
    }
}

/// Write an at-rule prelude (`@media print`, `@media(min-width:1px)`), spacing
/// the query only when it does not open with a parenthesis.
pub fn write_at_rule(
    out: &mut impl std::fmt::Write,
    kind: AtRuleKind,
    query: &str,
) -> std::fmt::Result {
    write!(out, "@{kind}")?;
    if !query.starts_with('(') {
        out.write_char(' ')?;
    }
    out.write_str(query)
}

impl Display for StyleSelector {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            StyleSelector::Selector(value) | StyleSelector::Global(value, _) => f.write_str(value),
            StyleSelector::At {
                kind,
                query,
                selector,
                outer,
                ..
            } => {
                for rule in outer {
                    write_at_rule(f, rule.kind, &rule.query)?;
                    f.write_str(" ")?;
                }
                write_at_rule(f, *kind, query)?;
                if let Some(selector) = selector {
                    write!(f, " {selector}")?;
                }
                Ok(())
            }
        }
    }
}

impl StyleSelector {
    /// The selector string used for class-name generation, borrowing when possible.
    ///
    /// `Selector`/`Global` variants have a `Display` impl that writes their inner
    /// `String` verbatim, so they can be handed out as a borrowed `&str` with zero
    /// allocation. Only the `At` variant needs the formatted owned string. This
    /// yields bytes identical to `self.to_string()` for every variant.
    #[must_use]
    pub fn as_class_str(&self) -> std::borrow::Cow<'_, str> {
        match self {
            StyleSelector::Selector(value) | StyleSelector::Global(value, _) => {
                std::borrow::Cow::Borrowed(value)
            }
            StyleSelector::At { .. } => std::borrow::Cow::Owned(self.to_string()),
        }
    }

    /// Nest a selector `template` (e.g. `&:hover`, `:root[data-theme=dark] &`)
    /// inside `parent`, substituting the parent's selector for `&`.
    #[must_use]
    pub fn nest_selector(parent: Option<&Self>, template: &str) -> Self {
        match parent {
            None => Self::Selector(template.to_string()),
            Some(Self::Selector(selector)) => Self::Selector(template.replace('&', selector)),
            Some(Self::Global(selector, file)) => {
                Self::Global(template.replace('&', selector), file.clone())
            }
            Some(Self::At {
                kind,
                query,
                selector,
                outer,
                file,
            }) => Self::At {
                kind: *kind,
                query: query.clone(),
                selector: Some(
                    selector
                        .as_deref()
                        .map_or_else(|| template.to_string(), |s| template.replace('&', s)),
                ),
                outer: outer.clone(),
                file: file.clone(),
            },
        }
    }

    /// Wrap `parent` (the bare class when `None`) in an at-rule. Nested
    /// `@media` rules fold into one query where possible; `None` means the
    /// combined condition can never match, so the styles must be dropped.
    #[must_use]
    pub fn nest_at_rule(parent: Option<&Self>, kind: AtRuleKind, query: &str) -> Option<Self> {
        let query = normalize_query(query);
        let (selector, outer, file) = match parent {
            None => (None, vec![], None),
            Some(Self::Selector(selector)) => (Some(selector.clone()), vec![], None),
            Some(Self::Global(selector, file)) => {
                (Some(selector.clone()), vec![], Some(file.clone()))
            }
            Some(Self::At {
                kind: parent_kind,
                query: parent_query,
                selector,
                outer,
                file,
            }) => {
                if *parent_kind == AtRuleKind::Media && kind == AtRuleKind::Media {
                    match combine_media_queries(parent_query, &query) {
                        MediaCombination::Merged(query) => {
                            return Some(Self::At {
                                kind,
                                query,
                                selector: selector.clone(),
                                outer: outer.clone(),
                                file: file.clone(),
                            });
                        }
                        MediaCombination::Never => return None,
                        MediaCombination::Nest => {}
                    }
                }
                let mut outer = outer.clone();
                outer.push(AtRule {
                    kind: *parent_kind,
                    query: parent_query.clone(),
                });
                (selector.clone(), outer, file.clone())
            }
        };
        Some(Self::At {
            kind,
            query,
            selector,
            outer,
            file,
        })
    }
}

/// Rank a `Global` selector's pseudo suffix by scanning `SELECTOR_ORDER` once.
///
/// "Last matching entry wins": the value of the last table token contained in
/// `order_suffix` (default 0). Computes each side's order in a single pass
/// instead of re-scanning the table per comparison.
#[must_use]
pub fn global_selector_order(order_suffix: &str) -> u8 {
    let mut order_value = 0;
    for (order, value) in SELECTOR_ORDER {
        if order_suffix.contains(order) {
            order_value = value;
        }
    }
    order_value
}

#[must_use]
pub fn get_selector_order(selector: &str) -> u8 {
    // Extract the part after the single '&' (avoid String allocation).
    // Single fused scan: record the first '&' byte index and detect a second '&'
    // in the same pass; slice the tail directly only when exactly one '&' was
    // seen (exactly one '&' ⇒ part after it), else use the whole selector.
    let mut first_amp: Option<usize> = None;
    let mut saw_second = false;
    for (i, b) in selector.bytes().enumerate() {
        if b == b'&' {
            if first_amp.is_none() {
                first_amp = Some(i);
            } else {
                saw_second = true;
                break;
            }
        }
    }
    let t: &str = match first_amp {
        Some(i) if !saw_second => &selector[i + 1..],
        _ => selector,
    };

    // First, try to find the order in the table (for regular selectors like &:hover).
    // Every SELECTOR_ORDER key starts with `:`, so a tail that does not begin with
    // `:` (e.g. `&`, or a group/global pattern) can never equal any key — gate the
    // 6-entry linear probe behind that single-byte check. Byte-identical result.
    if t.starts_with(':')
        && let Some((_, order)) = SELECTOR_ORDER.iter().find(|(k, _)| *k == t)
    {
        return *order;
    }

    // For group selectors like ":is([role=group],[data-group]):hover &", the pseudo-selector is before &
    // Check if the selector ends with " &" (group pattern) and contains a known pseudo-selector.
    // Every SELECTOR_ORDER key is a `:`-prefixed token with no interior `:`, so the pseudo suffix,
    // if any, is exactly the slice from the LAST `:` onward. Locate it once via `rfind(':')` and
    // compare that single slice against the table by equality, collapsing 6 `ends_with` tail-walks
    // into one `rfind` + up-to-6 `==`. Byte-identical: each key is a distinct suffix, so the first
    // (and only) `ends_with` match the previous loop could return equals the single-slice equality.
    if let Some(before_ampersand) = selector.strip_suffix(" &")
        && let Some(colon) = before_ampersand.rfind(':')
    {
        let suffix = &before_ampersand[colon..];
        if let Some((_, order)) = SELECTOR_ORDER.iter().find(|(k, _)| *k == suffix) {
            return *order;
        }
    }

    if t.starts_with('&') { 0 } else { 99 }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case("hover", StyleSelector::Selector("&:hover".to_string()))]
    #[case("focusVisible", StyleSelector::Selector("&:focus-visible".to_string()))]
    #[case("group-hover", StyleSelector::Selector(":is([role=group],[data-group]):hover &".to_string()))]
    #[case("group-focus-visible", StyleSelector::Selector(":is([role=group],[data-group]):focus-visible &".to_string()))]
    #[case("group-1", StyleSelector::Selector(":is([role=group],[data-group]):1 &".to_string()))]
    #[case(["theme-dark", "placeholder"], StyleSelector::Selector(":root[data-theme=dark] &::placeholder".to_string()))]
    #[case(["theme-dark", "&:hover"], StyleSelector::Selector(":root[data-theme=dark] &:hover".to_string()))]
    #[case("theme-light", StyleSelector::Selector(":root[data-theme=light] &".to_string()))]
    #[case("*[aria=disabled='true'] &:hover", StyleSelector::Selector("*[aria=disabled='true'] &:hover".to_string()))]
    fn test_style_selector(
        #[case] input: impl Into<StyleSelector>,
        #[case] expected: StyleSelector,
    ) {
        assert_eq!(input.into(), expected);
    }

    #[rstest]
    #[case(StyleSelector::Selector("&:hover".to_string()), "&:hover")]
    #[case(StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "screen and (max-width: 600px)".to_string(),
            selector: None, outer: vec![], file: None,
        },
        "@media screen and (max-width: 600px)"
    )]
    #[case(StyleSelector::At {
            kind: AtRuleKind::Supports,
            query: "(display: grid)".to_string(),
            selector: None, outer: vec![], file: None,
        },
        "@supports(display: grid)"
    )]
    #[case(StyleSelector::At {
            kind: AtRuleKind::Container,
            query: "(min-width: 768px)".to_string(),
            selector: None, outer: vec![], file: None,
        },
        "@container(min-width: 768px)"
    )]
    #[case(StyleSelector::At {
            kind: AtRuleKind::Container,
            query: "sidebar (min-width: 400px)".to_string(),
            selector: None, outer: vec![], file: None,
        },
        "@container sidebar (min-width: 400px)"
    )]
    #[case(StyleSelector::Global(":root[data-theme=dark]".to_string(), "file.rs".to_string()), ":root[data-theme=dark]")]
    #[case(StyleSelector::At {
            kind: AtRuleKind::Layer,
            query: "reset".to_string(),
            selector: None, outer: vec![], file: None,
        },
        "@layer reset"
    )]
    fn test_style_selector_display(#[case] selector: StyleSelector, #[case] expected: &str) {
        let output = format!("{selector}");
        assert_eq!(output, expected);
    }

    #[rstest]
    #[case(
        StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "screen".to_string(),
            selector: None, outer: vec![], file: None,
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
        StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "a".to_string(),
            selector: None, outer: vec![], file: None,
        },
        StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "b".to_string(),
            selector: None, outer: vec![], file: None,
        },
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "(min-width: 768px)".to_string(),
            selector: None, outer: vec![], file: None,
        },
        StyleSelector::At {
            kind: AtRuleKind::Supports,
            query: "(display: grid)".to_string(),
            selector: None, outer: vec![], file: None,
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
        StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "screen".to_string(),
            selector: None, outer: vec![], file: None,
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
    // Group selector ordering tests - _groupHover should come before _groupActive
    #[case(
        StyleSelector::Selector(":is([role=group],[data-group]):hover &".to_string()),
        StyleSelector::Selector(":is([role=group],[data-group]):active &".to_string()),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Selector(":is([role=group],[data-group]):focus-visible &".to_string()),
        StyleSelector::Selector(":is([role=group],[data-group]):focus &".to_string()),
        std::cmp::Ordering::Less
    )]
    #[case(
        StyleSelector::Selector(":is([role=group],[data-group]):active &".to_string()),
        StyleSelector::Selector(":is([role=group],[data-group]):hover &".to_string()),
        std::cmp::Ordering::Greater
    )]
    #[case(
        StyleSelector::Selector(":is([role=group],[data-group]):hover &".to_string()),
        StyleSelector::Selector(":is([role=group],[data-group]):hover &".to_string()),
        std::cmp::Ordering::Equal
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
    // Group selectors - pseudo-selector is before & (legacy single-form, still
    // accepted by the ordering function so older sheets continue to sort correctly)
    #[case("*[role=group]:hover &", 0)]
    #[case("*[role=group]:focus-visible &", 1)]
    #[case("*[role=group]:focus &", 2)]
    #[case("*[role=group]:active &", 3)]
    #[case("*[role=group]:selected &", 4)]
    #[case("*[role=group]:disabled &", 5)]
    #[case("*[role=group]:not-exist &", 99)]
    // Group selectors - new `:is()` form emitted by `From<&str>::from("group-*")`.
    // Ordering must rank these identically to the legacy single-form. The pseudo
    // sits after `:is(...)` so the suffix-based ordering logic remains stable
    // regardless of the comma-separated attribute list nested inside `:is()`.
    #[case(":is([role=group],[data-group]):hover &", 0)]
    #[case(":is([role=group],[data-group]):focus-visible &", 1)]
    #[case(":is([role=group],[data-group]):focus &", 2)]
    #[case(":is([role=group],[data-group]):active &", 3)]
    #[case(":is([role=group],[data-group]):selected &", 4)]
    #[case(":is([role=group],[data-group]):disabled &", 5)]
    #[case(":is([role=group],[data-group]):not-exist &", 99)]
    fn test_get_selector_order(#[case] selector: &str, #[case] expected: u8) {
        assert_eq!(get_selector_order(selector), expected);
    }

    /// Invariants of the `:is()` group selector emission:
    ///   1. Exactly one CSS selector clause (no top-level comma)
    ///   2. `:is()` argument list contains both `[role=group]` (deprecated, removed
    ///      in v2) and `[data-group]` (new, preferred)
    ///   3. Ends with `<pseudo> &` so `get_selector_order` ranks it correctly
    ///
    /// This pins the emission shape so a future refactor cannot silently drop the
    /// legacy `[role=group]` half (breaking backward compat) or split the selector
    /// back into two top-level clauses (re-introducing the fragile suffix-only
    /// ordering risk).
    #[rstest]
    #[case("group-hover", ":hover")]
    #[case("group-focus-visible", ":focus-visible")]
    #[case("group-focus", ":focus")]
    #[case("group-active", ":active")]
    #[case("group-disabled", ":disabled")]
    fn test_group_selector_is_form_invariants(
        #[case] input: &str,
        #[case] expected_pseudo_suffix: &str,
    ) {
        let StyleSelector::Selector(rendered) = StyleSelector::from(input) else {
            panic!("group-* should produce StyleSelector::Selector");
        };
        // (1) single clause: no comma outside the `:is()` argument list
        let Some((_, after_is)) = rendered.rsplit_once(')') else {
            panic!("rendered selector must contain `)` from :is(): `{rendered}`");
        };
        assert!(
            !after_is.contains(','),
            "expected single top-level clause, got `{rendered}`"
        );
        // (2) both legacy and new attribute selectors present inside :is()
        assert!(
            rendered.contains("[role=group]"),
            "missing legacy attribute in `{rendered}`"
        );
        assert!(
            rendered.contains("[data-group]"),
            "missing new attribute in `{rendered}`"
        );
        // (3) shape ends with `<pseudo> &`
        let before_ampersand = rendered.trim_end_matches('&').trim_end();
        assert!(
            before_ampersand.ends_with(expected_pseudo_suffix),
            "selector `{rendered}` does not end with pseudo `{expected_pseudo_suffix}`"
        );
    }

    fn at(kind: AtRuleKind, query: &str, selector: Option<&str>) -> StyleSelector {
        StyleSelector::At {
            kind,
            query: query.to_string(),
            selector: selector.map(str::to_string),
            outer: vec![],
            file: None,
        }
    }

    #[rstest]
    #[case(None, "&:hover", StyleSelector::Selector("&:hover".to_string()))]
    #[case(
        Some(StyleSelector::Selector("&:hover".to_string())),
        ":root[data-theme=dark] &",
        StyleSelector::Selector(":root[data-theme=dark] &:hover".to_string())
    )]
    #[case(
        Some(StyleSelector::Global("body".to_string(), "a.tsx".to_string())),
        "&:hover",
        StyleSelector::Global("body:hover".to_string(), "a.tsx".to_string())
    )]
    #[case(
        Some(at(AtRuleKind::Media, "print", None)),
        "&:hover",
        at(AtRuleKind::Media, "print", Some("&:hover"))
    )]
    #[case(
        Some(at(AtRuleKind::Media, "print", Some("&:focus"))),
        "&:hover",
        at(AtRuleKind::Media, "print", Some("&:focus:hover"))
    )]
    fn test_nest_selector(
        #[case] parent: Option<StyleSelector>,
        #[case] template: &str,
        #[case] expected: StyleSelector,
    ) {
        assert_eq!(
            StyleSelector::nest_selector(parent.as_ref(), template),
            expected
        );
    }

    #[rstest]
    #[case(
        None,
        AtRuleKind::Media,
        "(prefers-reduced-motion: reduce)",
        Some(at(AtRuleKind::Media, "(prefers-reduced-motion:reduce)", None))
    )]
    #[case(
        Some(StyleSelector::Selector("&:hover".to_string())),
        AtRuleKind::Media,
        "print",
        Some(at(AtRuleKind::Media, "print", Some("&:hover")))
    )]
    #[case(
        Some(StyleSelector::Global("body".to_string(), "a.tsx".to_string())),
        AtRuleKind::Media,
        "print",
        Some(StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "print".to_string(),
            selector: Some("body".to_string()),
            outer: vec![],
            file: Some("a.tsx".to_string()),
        })
    )]
    #[case(
        Some(at(AtRuleKind::Media, "print", Some("&:hover"))),
        AtRuleKind::Media,
        "(prefers-reduced-motion:reduce)",
        Some(at(
            AtRuleKind::Media,
            "print and (prefers-reduced-motion:reduce)",
            Some("&:hover")
        ))
    )]
    #[case(
        Some(at(AtRuleKind::Media, "print", None)),
        AtRuleKind::Media,
        "screen",
        None
    )]
    #[case(
        Some(at(AtRuleKind::Media, "print", None)),
        AtRuleKind::Supports,
        "(display: grid)",
        Some(StyleSelector::At {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".to_string(),
            selector: None,
            outer: vec![AtRule { kind: AtRuleKind::Media, query: "print".to_string() }],
            file: None,
        })
    )]
    #[case(
        Some(at(AtRuleKind::Media, "not print and (color)", None)),
        AtRuleKind::Media,
        "(hover:none)",
        Some(StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "(hover:none)".to_string(),
            selector: None,
            outer: vec![AtRule { kind: AtRuleKind::Media, query: "not print and (color)".to_string() }],
            file: None,
        })
    )]
    fn test_nest_at_rule(
        #[case] parent: Option<StyleSelector>,
        #[case] kind: AtRuleKind,
        #[case] query: &str,
        #[case] expected: Option<StyleSelector>,
    ) {
        assert_eq!(
            StyleSelector::nest_at_rule(parent.as_ref(), kind, query),
            expected
        );
    }

    #[test]
    fn test_nested_at_rule_display_and_order() {
        let nested = StyleSelector::At {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".to_string(),
            selector: Some("&:hover".to_string()),
            outer: vec![AtRule {
                kind: AtRuleKind::Media,
                query: "print".to_string(),
            }],
            file: None,
        };
        assert_eq!(
            nested.to_string(),
            "@media print @supports(display:grid) &:hover"
        );
        assert_eq!(
            nested.as_class_str(),
            "@media print @supports(display:grid) &:hover"
        );

        let plain = at(AtRuleKind::Supports, "(display:grid)", Some("&:hover"));
        assert_eq!(plain.cmp(&nested), std::cmp::Ordering::Less);
        let global = StyleSelector::At {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".to_string(),
            selector: Some("&:hover".to_string()),
            outer: vec![],
            file: Some("a.tsx".to_string()),
        };
        assert_eq!(plain.cmp(&global), std::cmp::Ordering::Less);
        assert_eq!(
            optimize_selector(global.clone()),
            global,
            "optimizing keeps the enclosing rules and file"
        );
    }

    #[test]
    fn test_media_shorthand_selector() {
        assert_eq!(
            StyleSelector::from("motion-reduce"),
            at(AtRuleKind::Media, "(prefers-reduced-motion:reduce)", None)
        );
        assert_eq!(
            StyleSelector::from("print"),
            at(AtRuleKind::Media, "print", None)
        );
        assert_eq!(
            StyleSelector::from("speech"),
            StyleSelector::Selector("&:speech".to_string())
        );
    }

    #[rstest]
    #[case(AtRuleKind::Media, "media")]
    #[case(AtRuleKind::Supports, "supports")]
    #[case(AtRuleKind::Container, "container")]
    #[case(AtRuleKind::Layer, "layer")]
    fn test_at_rule_kind_display(#[case] kind: AtRuleKind, #[case] expected: &str) {
        assert_eq!(format!("{kind}"), expected);
    }

    #[rstest]
    #[case("media", AtRuleKind::Media)]
    #[case("supports", AtRuleKind::Supports)]
    #[case("container", AtRuleKind::Container)]
    #[case("layer", AtRuleKind::Layer)]
    fn test_at_rule_kind_from_str(#[case] input: &str, #[case] expected: AtRuleKind) {
        assert_eq!(AtRuleKind::from(input), expected);
    }

    #[test]
    #[should_panic(expected = "internal error: entered unreachable code")]
    fn test_at_rule_kind_from_str_unknown() {
        let _ = AtRuleKind::from("unknown");
    }
}
