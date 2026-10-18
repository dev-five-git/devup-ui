//! The definitions a project's Tailwind CSS file makes: `@theme` variables,
//! `@utility` utilities and `@custom-variant` variants, read at build time
//!
//! A definition that cannot be read exactly is left out, so the classes that
//! use it stay as written; a file that cannot be read at all defines nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{LazyLock, RwLock};

use crate::style_selector::AtRuleKind;

/// A `@custom-variant`: the selectors the element matches, any one of them
/// (`&` being the element), and the at-rule the styles go in
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomVariant {
    pub selectors: Vec<String>,
    pub at_rule: Option<(AtRuleKind, String)>,
}

/// One line of the body of an `@utility`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UtilityItem {
    /// `property: value`
    Declaration(String, String),
    /// `@apply a b c`: the utilities that are applied
    Apply(Vec<String>),
}

#[derive(Debug, Default)]
struct Definitions {
    theme: BTreeMap<String, String>,
    /// Namespaces `--ns-*: initial` empties, such as `--color-`
    resets: BTreeSet<String>,
    /// `--*: initial` empties every namespace
    reset_all: bool,
    utilities: BTreeMap<String, Vec<UtilityItem>>,
    /// `@utility name-*`, by `name`: declarations that read `--value()` and
    /// `--modifier()`
    functional: BTreeMap<String, Vec<(String, String)>>,
    variants: BTreeMap<String, CustomVariant>,
    keyframes: BTreeMap<String, String>,
}

static DEFINITIONS: LazyLock<RwLock<Definitions>> =
    LazyLock::new(|| RwLock::new(Definitions::default()));

/// One statement of a stylesheet
#[derive(Debug, PartialEq, Eq)]
enum Item {
    /// `prelude;`, which a declaration is as well
    Statement(String),
    /// `prelude { body }`
    Block(String, String),
}

/// `css` without its comments
fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start + 2..]
            .find("*/")
            .map_or("", |end| &rest[start + 2 + end + 2..]);
    }
    out.push_str(rest);
    out
}

/// The statements and blocks of `css`, `None` when its brackets or strings do
/// not balance
fn parse_items(css: &str) -> Option<Vec<Item>> {
    let mut items = Vec::new();
    let mut prelude = String::new();
    let mut body = String::new();
    let mut depth = 0usize;
    let mut parens = 0usize;
    let mut quote: Option<char> = None;
    for c in css.chars() {
        let target = if depth == 0 { &mut prelude } else { &mut body };
        if let Some(open) = quote {
            target.push(c);
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                target.push(c);
            }
            '(' => {
                parens += 1;
                target.push(c);
            }
            ')' => {
                parens = parens.checked_sub(1)?;
                target.push(c);
            }
            '{' if parens == 0 => {
                depth += 1;
                if depth > 1 {
                    body.push(c);
                }
            }
            '}' if parens == 0 => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    items.push(Item::Block(
                        prelude.trim().to_string(),
                        std::mem::take(&mut body),
                    ));
                    prelude.clear();
                } else {
                    body.push(c);
                }
            }
            ';' if parens == 0 && depth == 0 => {
                items.push(Item::Statement(prelude.trim().to_string()));
                prelude.clear();
            }
            _ => target.push(c),
        }
    }
    let balanced = depth == 0 && parens == 0 && quote.is_none();
    if balanced && !prelude.trim().is_empty() {
        items.push(Item::Statement(prelude.trim().to_string()));
    }
    balanced.then_some(items)
}

/// `value` without the whitespace CSS does not need
fn compact(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut pending_space = false;
    for c in value.trim().chars() {
        if c.is_whitespace() {
            pending_space = true;
            continue;
        }
        let tight = matches!(c, '{' | '}' | ':' | ';' | ',');
        if pending_space && !tight && !out.ends_with(['{', '}', ':', ';', ',']) {
            out.push(' ');
        }
        pending_space = false;
        out.push(c);
    }
    out
}

/// The declarations and `@apply` lines of `body`, `None` when it holds anything
/// else
fn utility_items(body: &str) -> Option<Vec<UtilityItem>> {
    parse_items(body)?
        .into_iter()
        .map(|item| match item {
            Item::Statement(statement) => match statement.strip_prefix("@apply") {
                Some(tokens) if tokens.starts_with(char::is_whitespace) => Some(
                    UtilityItem::Apply(tokens.split_whitespace().map(str::to_string).collect()),
                ),
                Some(_) => None,
                None => declaration(&statement)
                    .map(|(property, value)| UtilityItem::Declaration(property, value)),
            },
            Item::Block(..) => None,
        })
        .collect()
}

/// `name: value` as the pair it declares
fn declaration(statement: &str) -> Option<(String, String)> {
    let (name, value) = statement.split_once(':')?;
    let name = name.trim();
    let value = value.trim();
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'*'));
    (valid && !value.is_empty()).then(|| (name.to_string(), value.to_string()))
}

/// The declarations of `body`, `None` when it holds anything else
fn declarations(body: &str) -> Option<Vec<(String, String)>> {
    parse_items(body)?
        .into_iter()
        .map(|item| match item {
            Item::Statement(statement) => declaration(&statement),
            Item::Block(..) => None,
        })
        .collect()
}

impl Definitions {
    fn read_theme(&mut self, body: &str) -> Option<()> {
        for item in parse_items(body)? {
            match item {
                Item::Statement(statement) => {
                    let Some((name, value)) = declaration(&statement) else {
                        continue;
                    };
                    self.theme_variable(name, value);
                }
                Item::Block(prelude, block) => {
                    if let Some(name) = prelude.strip_prefix("@keyframes") {
                        let name = name.trim().to_string();
                        let rule = compact(&format!("@keyframes {name}{{{block}}}"));
                        self.keyframes.insert(name, rule);
                    }
                }
            }
        }
        Some(())
    }

    fn theme_variable(&mut self, name: String, value: String) {
        if !name.starts_with("--") {
            return;
        }
        if value == "initial" {
            if name == "--*" {
                self.reset_all = true;
            } else if let Some(namespace) = name.strip_suffix('*') {
                self.resets.insert(namespace.to_string());
            }
            return;
        }
        if name.contains('*') || value.contains("theme(") {
            return;
        }
        self.theme.insert(name, value);
    }

    fn read_utility(&mut self, name: &str, body: &str) {
        if name.is_empty() || name.contains(char::is_whitespace) {
            return;
        }
        if let Some(root) = name.strip_suffix("-*") {
            if let Some(declarations) =
                declarations(body).filter(|_| !root.is_empty() && !root.contains('*'))
            {
                self.functional.insert(root.to_string(), declarations);
            }
            return;
        }
        if name.contains('*') {
            return;
        }
        if let Some(items) = utility_items(body) {
            self.utilities.insert(name.to_string(), items);
        }
    }
    fn read_custom_variant(&mut self, rest: &str, body: Option<&str>) {
        let rest = rest.trim();
        let (name, definition) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let definition = definition.trim();
        let variant = match body {
            None => definition
                .strip_prefix('(')
                .and_then(|inner| inner.strip_suffix(')'))
                .map(selector_variant),
            Some(body) if definition.is_empty() => block_variant(body),
            Some(_) => None,
        };
        if let Some(variant) = variant.filter(|_| !name.is_empty()) {
            self.variants.insert(name.to_string(), variant);
        }
    }
}

/// The selectors of `(…)`, which must hold the element as `&`
fn selector_variant(selectors: &str) -> CustomVariant {
    CustomVariant {
        selectors: split_selectors(selectors),
        at_rule: None,
    }
}

/// `selectors` split at the commas outside any bracket
fn split_selectors(selectors: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in selectors.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(selectors[start..index].trim().to_string());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(selectors[start..].trim().to_string());
    parts
}

/// The variant a `@custom-variant name { … }` block holds: `@slot;` alone,
/// or one selector or at-rule around it
fn block_variant(body: &str) -> Option<CustomVariant> {
    match parse_items(body)?.as_slice() {
        [Item::Statement(statement)] if statement == "@slot" => Some(selector_variant("&")),
        [Item::Block(prelude, inner)] if compact(inner).trim_end_matches(';') == "@slot" => {
            around_slot(prelude)
        }
        _ => None,
    }
}

/// The variant `prelude { @slot; }` is: an at-rule or a selector
fn around_slot(prelude: &str) -> Option<CustomVariant> {
    for (name, kind) in [
        ("@media", AtRuleKind::Media),
        ("@supports", AtRuleKind::Supports),
    ] {
        if let Some(query) = prelude.strip_prefix(name) {
            return Some(CustomVariant {
                selectors: Vec::new(),
                at_rule: Some((kind, compact(query))),
            });
        }
    }
    (!prelude.starts_with('@')).then(|| selector_variant(prelude))
}
/// `var(--name)` written whole in `value`
fn sole_variable(value: &str) -> Option<&str> {
    value
        .strip_prefix("var(")
        .and_then(|inner| inner.strip_suffix(')'))
        .map(str::trim)
        .filter(|name| name.starts_with("--") && !name.contains([',', ' ', '(']))
}

fn read(css: &str) -> Option<Definitions> {
    let css = strip_comments(css);
    let mut definitions = Definitions::default();
    for item in parse_items(&css)? {
        match item {
            Item::Block(prelude, body) => {
                let (at, rest) = prelude
                    .split_once(char::is_whitespace)
                    .unwrap_or((prelude.as_str(), ""));
                match at {
                    "@theme" => definitions.read_theme(&body)?,
                    "@utility" => definitions.read_utility(rest.trim(), &body),
                    "@custom-variant" => definitions.read_custom_variant(rest, Some(&body)),
                    _ => {}
                }
            }
            Item::Statement(statement) => {
                if let Some(rest) = statement.strip_prefix("@custom-variant") {
                    definitions.read_custom_variant(rest, None);
                }
            }
        }
    }
    definitions.resolve_references();
    Some(definitions)
}

impl Definitions {
    /// A theme variable that is another one (`--color-brand: var(--color-mint)`)
    /// is that one's value, as the theme variables are not written out
    fn resolve_references(&mut self) {
        for _ in 0..8 {
            let snapshot = self.theme.clone();
            for value in self.theme.values_mut() {
                if let Some(target) = sole_variable(value).and_then(|name| snapshot.get(name)) {
                    value.clone_from(target);
                }
            }
        }
    }
}

/// Reads the definitions of the Tailwind CSS in `css`, which replace the ones
/// read before; `""` clears them
pub fn set_tailwind_css(css: &str) {
    let definitions = read(css).unwrap_or_default();
    if let Ok(mut registry) = DEFINITIONS.write() {
        *registry = definitions;
    }
}

fn with<T>(read: impl FnOnce(&Definitions) -> Option<T>) -> Option<T> {
    DEFINITIONS.read().ok().and_then(|registry| read(&registry))
}

/// The value of the theme variable `--<namespace>-<key>`
#[must_use]
pub fn theme_value(namespace: &str, key: &str) -> Option<String> {
    with(|definitions| {
        definitions
            .theme
            .get(&format!("--{namespace}-{key}"))
            .cloned()
    })
}

/// The value of the theme variable `--<name>`
#[must_use]
pub fn theme_variable(name: &str) -> Option<String> {
    with(|definitions| definitions.theme.get(&format!("--{name}")).cloned())
}

/// Whether the theme empties the default values of `--<namespace>-*`
#[must_use]
pub fn namespace_is_reset(namespace: &str) -> bool {
    with(|definitions| {
        Some(definitions.reset_all || definitions.resets.contains(&format!("--{namespace}-")))
    })
    .unwrap_or(false)
}

/// The keys of the theme variables `--<namespace>-<key>`
#[must_use]
pub fn theme_keys(namespace: &str) -> Vec<String> {
    let prefix = format!("--{namespace}-");
    with(|definitions| {
        Some(
            definitions
                .theme
                .keys()
                .filter_map(|name| name.strip_prefix(&prefix).map(str::to_string))
                .collect(),
        )
    })
    .unwrap_or_default()
}

/// The body of the `@utility` named `name`
#[must_use]
pub fn custom_utility(name: &str) -> Option<Vec<UtilityItem>> {
    with(|definitions| definitions.utilities.get(name).cloned())
}

/// The declarations of the functional `@utility` named `<root>-*`
#[must_use]
pub fn functional_utility(root: &str) -> Option<Vec<(String, String)>> {
    with(|definitions| definitions.functional.get(root).cloned())
}

/// The `@custom-variant` named `name`
#[must_use]
pub fn custom_variant(name: &str) -> Option<CustomVariant> {
    with(|definitions| definitions.variants.get(name).cloned())
}

/// The `@keyframes` rule the theme defines for `name`
#[must_use]
pub fn theme_keyframes(name: &str) -> Option<String> {
    with(|definitions| definitions.keyframes.get(name).cloned())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn theme_variables_are_read_by_namespace() {
        set_tailwind_css(
            "@import \"tailwindcss\";\n/* c */ @theme { --color-brand: #f00; --spacing-gutter: 3rem; --breakpoint-3xl: 120rem; --font-display: 'Inter', sans-serif; }",
        );
        assert_eq!(theme_value("color", "brand").as_deref(), Some("#f00"));
        assert_eq!(theme_value("spacing", "gutter").as_deref(), Some("3rem"));
        assert_eq!(theme_value("breakpoint", "3xl").as_deref(), Some("120rem"));
        assert_eq!(
            theme_value("font", "display").as_deref(),
            Some("'Inter', sans-serif")
        );
        assert_eq!(theme_value("color", "nothing"), None);
        assert_eq!(theme_variable("color-brand").as_deref(), Some("#f00"));
        assert_eq!(theme_keys("color"), vec!["brand"]);
        set_tailwind_css("");
        assert_eq!(theme_value("color", "brand"), None);
        assert_eq!(theme_keys("color"), Vec::<String>::new());
    }

    #[test]
    #[serial]
    fn theme_options_and_resets() {
        set_tailwind_css(
            "@theme inline { --color-*: initial; --color-a: red; --color-b: var(--color-a); --color-c: var(--color-b); --x-*: theme(foo); --color-d: theme(--color-a); }",
        );
        assert!(namespace_is_reset("color"));
        assert!(!namespace_is_reset("shadow"));
        assert_eq!(theme_value("color", "b").as_deref(), Some("red"));
        assert_eq!(theme_value("color", "c").as_deref(), Some("red"));
        assert_eq!(theme_value("color", "d"), None);
        set_tailwind_css("@theme { --*: initial; --color-a: red }");
        assert!(namespace_is_reset("shadow"));
        set_tailwind_css("@theme { --color-a: var(--other, red); color: red; --: 1; --a b: 2 }");
        assert_eq!(
            theme_value("color", "a").as_deref(),
            Some("var(--other, red)")
        );
        assert!(!namespace_is_reset("color"));
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn theme_keyframes_are_kept_compact() {
        set_tailwind_css(
            "@theme { --animate-wiggle: wiggle 1s ease-in-out infinite;\n @keyframes wiggle { 0%, 100% { transform: rotate(-3deg); } 50% { transform: rotate(3deg); } } @media print {} }",
        );
        assert_eq!(
            theme_keyframes("wiggle").as_deref(),
            Some(
                "@keyframes wiggle{0%,100%{transform:rotate(-3deg);}50%{transform:rotate(3deg);}}"
            )
        );
        assert_eq!(theme_keyframes("other"), None);
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn utilities_are_static_declarations() {
        set_tailwind_css(
            "@layer base { a { color: red } } @utility content-auto { content-visibility: auto; contain-intrinsic-size: 0 500px } @utility tab-* { tab-size: --value(integer); } @utility nested { &:hover { color: red } } @utility empty { } @utility two words { a: b } @utility btn { @apply px-4 py-2; color: red; } @utility bad-apply { @applyx a; } @utility any-* x { a: b } @utility -* { a: b } @utility a*b { a: b } @utility fn-* { a: b; &:hover { c: d } }",
        );
        assert_eq!(
            custom_utility("content-auto"),
            Some(vec![
                UtilityItem::Declaration("content-visibility".to_string(), "auto".to_string()),
                UtilityItem::Declaration(
                    "contain-intrinsic-size".to_string(),
                    "0 500px".to_string()
                ),
            ])
        );
        assert_eq!(custom_utility("tab-*"), None);
        assert_eq!(
            functional_utility("tab"),
            Some(vec![(
                "tab-size".to_string(),
                "--value(integer)".to_string()
            )])
        );
        assert_eq!(custom_utility("nested"), None);
        assert_eq!(custom_utility("empty"), Some(vec![]));
        assert_eq!(custom_utility("two"), None);
        assert_eq!(
            custom_utility("btn"),
            Some(vec![
                UtilityItem::Apply(vec!["px-4".to_string(), "py-2".to_string()]),
                UtilityItem::Declaration("color".to_string(), "red".to_string()),
            ])
        );
        assert_eq!(custom_utility("bad-apply"), None);
        assert_eq!(functional_utility("any"), None);
        assert_eq!(functional_utility("fn"), None);
        assert_eq!(functional_utility(""), None);
        assert_eq!(custom_utility("a*b"), None);
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn custom_variants() {
        set_tailwind_css(
            "@custom-variant theme-midnight (&:where([data-theme=\"midnight\"] *));\n@custom-variant dark { &:where(.dark, .dark *) { @slot; } }\n@custom-variant pointer-hover { @media (hover: hover) { @slot; } }\n@custom-variant wide { @supports (display: grid) { @slot; } }\n@custom-variant any (&:hover, &:focus);\n@custom-variant plain { @slot; }\n@custom-variant bad { &:hover { color: red; } }\n@custom-variant worse { &:hover { &:focus { @slot; } } @slot; }\n@custom-variant bare;\n@custom-variant (&:hover);\n@custom-variant named-block extra { &:hover { @slot; } }\n@custom-variant at { @layer x { @slot; } }",
        );
        assert_eq!(
            custom_variant("theme-midnight").unwrap().selectors,
            vec!["&:where([data-theme=\"midnight\"] *)"]
        );
        assert_eq!(
            custom_variant("dark").unwrap().selectors,
            vec!["&:where(.dark, .dark *)"]
        );
        assert_eq!(
            custom_variant("pointer-hover").unwrap(),
            CustomVariant {
                selectors: vec![],
                at_rule: Some((AtRuleKind::Media, "(hover:hover)".to_string())),
            }
        );
        assert_eq!(
            custom_variant("wide").unwrap().at_rule,
            Some((AtRuleKind::Supports, "(display:grid)".to_string()))
        );
        assert_eq!(
            custom_variant("any").unwrap().selectors,
            vec!["&:hover", "&:focus"]
        );
        assert_eq!(custom_variant("plain").unwrap().selectors, vec!["&"]);
        for name in ["bad", "worse", "bare", "named-block", "at", ""] {
            assert_eq!(custom_variant(name), None, "{name}");
        }
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn a_file_that_cannot_be_read_defines_nothing() {
        for css in [
            "@theme { --color-a: red;",
            "@theme { --color-a: red; }}",
            "@theme { --color-a: url(\"a }",
            "@utility a { b: c) }",
            "@theme { --color-a: (red }",
        ] {
            set_tailwind_css("@theme { --color-keep: red }");
            set_tailwind_css(css);
            assert_eq!(theme_value("color", "a"), None, "{css}");
            assert_eq!(theme_value("color", "keep"), None, "{css}");
        }
        set_tailwind_css("");
    }

    #[test]
    fn items_and_text_helpers() {
        assert_eq!(strip_comments("a/* x */b/* open"), "ab");
        assert_eq!(
            parse_items("@import \"a;b\"; .a { b: c }").unwrap(),
            vec![
                Item::Statement("@import \"a;b\"".to_string()),
                Item::Block(".a".to_string(), " b: c ".to_string()),
            ]
        );
        assert_eq!(
            parse_items("a: b").unwrap(),
            vec![Item::Statement("a: b".to_string())]
        );
        assert_eq!(parse_items("  ").unwrap(), vec![]);
        assert_eq!(compact(" a  { b : c ; } "), "a{b:c;}");
        assert_eq!(compact("a   b"), "a b");
        assert_eq!(split_selectors("&:a(b, c), &d"), vec!["&:a(b, c)", "&d"]);
        assert_eq!(sole_variable("var(--a)"), Some("--a"));
        assert_eq!(sole_variable("var(--a, red)"), None);
        assert_eq!(sole_variable("red"), None);
        assert_eq!(declaration("a"), None);
        assert_eq!(declaration("a:"), None);
        assert_eq!(declaration("a b: c"), None);
        assert_eq!(declarations("a: b; c { d: e }"), None);
    }
}
