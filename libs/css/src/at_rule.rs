//! At-rule preludes: media shorthands, query normalization, media query
//! combination, and the deterministic order at-rule blocks are emitted in.

use crate::style_selector::AtRuleKind;
use crate::utils::to_kebab_case;

/// Media shorthand props and the media query each one stands for, keyed by the
/// kebab-cased name shared with the matching Tailwind variant.
const MEDIA_SHORTHANDS: [(&str, &str); 10] = [
    ("all", "all"),
    ("contrast-less", "(prefers-contrast:less)"),
    ("contrast-more", "(prefers-contrast:more)"),
    ("forced-colors", "(forced-colors:active)"),
    ("landscape", "(orientation:landscape)"),
    ("motion-reduce", "(prefers-reduced-motion:reduce)"),
    ("motion-safe", "(prefers-reduced-motion:no-preference)"),
    ("portrait", "(orientation:portrait)"),
    ("print", "print"),
    ("screen", "screen"),
];

/// The media query a shorthand prop (`print`, `motionReduce`, `motion-reduce`, …)
/// stands for.
#[must_use]
pub fn media_shorthand_query(name: &str) -> Option<&'static str> {
    let name = to_kebab_case(name);
    MEDIA_SHORTHANDS
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, query)| *query)
}

/// Split an at-rule key such as `@media print` into its kind and prelude.
#[must_use]
pub fn split_at_rule_key(key: &str) -> Option<(AtRuleKind, &str)> {
    let rest = key.trim().strip_prefix('@')?;
    [
        ("media", AtRuleKind::Media),
        ("supports", AtRuleKind::Supports),
        ("container", AtRuleKind::Container),
    ]
    .into_iter()
    .find_map(|(name, kind)| {
        let query = rest.strip_prefix(name)?;
        (query.starts_with([' ', '(']) && !query.trim().is_empty()).then_some((kind, query.trim()))
    })
}

const fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '%') || !c.is_ascii()
}

fn leading_word(text: &str) -> &str {
    let end = text
        .char_indices()
        .find(|(_, c)| !is_word_char(*c))
        .map_or(text.len(), |(index, _)| index);
    &text[..end]
}

fn trailing_word(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map_or(text.len(), |(index, _)| index);
    &text[start..]
}

fn is_keyword(word: &str) -> bool {
    ["and", "or", "not", "only"]
        .iter()
        .any(|keyword| word.eq_ignore_ascii_case(keyword))
}

/// Whether `text` ends with a standalone `and`/`or`/`not`/`only` keyword, which
/// needs a space before a following `(` so it is not read as a function.
fn ends_with_keyword(text: &str) -> bool {
    let word = trailing_word(text);
    is_keyword(word)
        && text[..text.len() - word.len()]
            .chars()
            .next_back()
            .is_none_or(|c| matches!(c, ' ' | '(' | ')' | ','))
}

/// Whether the whitespace between `prev` and `next` must survive as one space.
const fn keeps_space(prev: char, next: char, in_function: bool) -> bool {
    if matches!(prev, '(' | ',') || matches!(next, ')' | ',') {
        return false;
    }
    in_function
        || !(matches!(prev, ':' | '<' | '>' | '=' | ')') || matches!(next, ':' | '<' | '>' | '='))
}

/// Rewrite an at-rule prelude into its shortest equivalent spelling.
///
/// Whitespace is dropped wherever the tokens stay unambiguous (`(min-width: 1px)`
/// becomes `(min-width:1px)`, `) and (` becomes `)and (`) and kept where CSS
/// needs it: between words and before a `(` that follows a word, since `and(`
/// or `sidebar(` would tokenize as a function. A missing space in `and(` is
/// added back. Function arguments (`calc(1px + 2px)`, `selector(a :hover)`)
/// and quoted strings keep their significant whitespace.
#[must_use]
pub fn normalize_query(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    // One entry per open parenthesis: `true` when it opened a function, whose
    // arguments can contain significant whitespace.
    let mut parens: Vec<bool> = Vec::new();
    let mut pending_space = false;
    let mut chars = query.chars();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            pending_space = true;
            continue;
        }
        let in_function = parens.last().copied().unwrap_or(false);
        let keyword_paren = c == '(' && !in_function && ends_with_keyword(&out);
        let function_paren = c == '('
            && !pending_space
            && !keyword_paren
            && out.chars().next_back().is_some_and(is_word_char);
        if let Some(prev) = out.chars().next_back()
            && (keyword_paren || (pending_space && keeps_space(prev, c, in_function)))
        {
            out.push(' ');
        }
        pending_space = false;
        out.push(c);
        match c {
            '(' => parens.push(function_paren),
            ')' => {
                parens.pop();
            }
            '"' | '\'' => {
                let mut escaped = false;
                for inner in chars.by_ref() {
                    out.push(inner);
                    if escaped {
                        escaped = false;
                    } else if inner == '\\' {
                        escaped = true;
                    } else if inner == c {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Result of folding two media queries into one.
#[derive(Debug, PartialEq, Eq)]
pub enum MediaCombination {
    /// Both queries hold exactly when this single query list holds.
    Merged(String),
    /// The queries can never hold together (e.g. `print` inside `screen`).
    Never,
    /// No single query expresses both; emit nested `@media` blocks instead.
    Nest,
}

struct MediaQuery<'a> {
    negated: bool,
    only: bool,
    media_type: Option<&'a str>,
    condition: Option<&'a str>,
}

fn split_list(query: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in query.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(query[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(query[start..].trim());
    parts
}

fn parse_media_query(query: &str) -> Option<MediaQuery<'_>> {
    let condition_only = MediaQuery {
        negated: false,
        only: false,
        media_type: None,
        condition: Some(query),
    };
    if query.starts_with('(') {
        return Some(condition_only);
    }
    let first = leading_word(query);
    let rest = &query[first.len()..];
    let (negated, only, typed) = if first.eq_ignore_ascii_case("not") {
        if rest.starts_with(" (") {
            return Some(condition_only);
        }
        (true, false, rest.strip_prefix(' ')?)
    } else if first.eq_ignore_ascii_case("only") {
        (false, true, rest.strip_prefix(' ')?)
    } else {
        (false, false, query)
    };
    let media_type = leading_word(typed);
    if media_type.is_empty() || is_keyword(media_type) {
        return None;
    }
    let after = &typed[media_type.len()..];
    let condition = if after.is_empty() {
        None
    } else {
        Some(
            after
                .strip_prefix(" and ")
                .filter(|condition| condition.starts_with('(') || condition.starts_with("not "))?,
        )
    };
    Some(MediaQuery {
        negated,
        only,
        media_type: Some(media_type),
        condition,
    })
}

/// Turn `not print` into `screen` (and back), the only negations with a
/// positive spelling; anything else cannot be folded into another query.
const fn resolve_negation(query: MediaQuery<'_>) -> Result<MediaQuery<'_>, MediaCombination> {
    if !query.negated {
        return Ok(query);
    }
    if query.condition.is_some() {
        return Err(MediaCombination::Nest);
    }
    let complement = match query.media_type {
        Some(media_type) if media_type.eq_ignore_ascii_case("print") => "screen",
        Some(media_type) if media_type.eq_ignore_ascii_case("screen") => "print",
        Some(media_type) if media_type.eq_ignore_ascii_case("all") => {
            return Err(MediaCombination::Never);
        }
        _ => return Err(MediaCombination::Nest),
    };
    Ok(MediaQuery {
        negated: false,
        only: false,
        media_type: Some(complement),
        condition: None,
    })
}

fn has_top_level_or(condition: &str) -> bool {
    let mut depth = 0usize;
    let mut previous = ' ';
    for (index, c) in condition.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0
                && !is_word_char(previous)
                && leading_word(&condition[index..]).eq_ignore_ascii_case("or") =>
            {
                return true;
            }
            _ => {}
        }
        previous = c;
    }
    false
}

fn specific_type(media_type: Option<&str>) -> Option<&str> {
    media_type.filter(|media_type| !media_type.eq_ignore_ascii_case("all"))
}

fn combine_single(outer: MediaQuery<'_>, inner: MediaQuery<'_>) -> MediaCombination {
    let (outer, inner) = match (resolve_negation(outer), resolve_negation(inner)) {
        (Ok(outer), Ok(inner)) => (outer, inner),
        (Err(MediaCombination::Never), _) | (_, Err(MediaCombination::Never)) => {
            return MediaCombination::Never;
        }
        _ => return MediaCombination::Nest,
    };
    let media_type = match (
        specific_type(outer.media_type),
        specific_type(inner.media_type),
    ) {
        (None, media_type) | (media_type, None) => media_type,
        (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => Some(a),
        _ => return MediaCombination::Never,
    };
    let conditions: Vec<&str> = [outer.condition, inner.condition]
        .into_iter()
        .flatten()
        .collect();
    let joined = conditions.len() > 1;

    let mut result = String::new();
    if let Some(media_type) = media_type {
        if outer.only || inner.only {
            result.push_str("only ");
        }
        result.push_str(media_type);
        if !conditions.is_empty() {
            result.push_str(" and ");
        }
    }
    for (index, condition) in conditions.iter().enumerate() {
        if index > 0 {
            result.push_str("and ");
        }
        let wrap = (has_top_level_or(condition) && (joined || media_type.is_some()))
            || (joined && leading_word(condition).eq_ignore_ascii_case("not"));
        if wrap {
            result.push('(');
            result.push_str(condition);
            result.push(')');
        } else {
            result.push_str(condition);
        }
    }
    if result.is_empty() {
        result.push_str("all");
    }
    MediaCombination::Merged(result)
}

/// Fold `inner` into `outer` so one `@media` holds exactly when both would.
///
/// Both preludes must already be normalized. Query lists distribute over each
/// other, `not print`/`not screen` become `screen`/`print`, conflicting media
/// types collapse to [`MediaCombination::Never`], and anything that has no
/// single-query spelling (such as `not print and (color)`) asks for nesting.
#[must_use]
pub fn combine_media_queries(outer: &str, inner: &str) -> MediaCombination {
    let mut merged: Vec<String> = Vec::new();
    for outer_query in split_list(outer) {
        for inner_query in split_list(inner) {
            let (Some(outer_query), Some(inner_query)) = (
                parse_media_query(outer_query),
                parse_media_query(inner_query),
            ) else {
                return MediaCombination::Nest;
            };
            match combine_single(outer_query, inner_query) {
                MediaCombination::Merged(query) => {
                    if !merged.contains(&query) {
                        merged.push(query);
                    }
                }
                MediaCombination::Never => {}
                MediaCombination::Nest => return MediaCombination::Nest,
            }
        }
    }
    if merged.is_empty() {
        MediaCombination::Never
    } else {
        MediaCombination::Merged(merged.join(","))
    }
}

fn length_px(value: &str) -> Option<i64> {
    let number_end = value
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.'))
        .map_or(value.len(), |(index, _)| index);
    let number: f64 = value[..number_end].parse().ok()?;
    let unit = &value[number_end..];
    let scale = if unit.eq_ignore_ascii_case("em") || unit.eq_ignore_ascii_case("rem") {
        16.0
    } else {
        1.0
    };
    Some((number * scale * 100.0).round() as i64)
}

/// The first width bound of a normalized query: `(true, px)` for a lower bound
/// (`min-width`, `width>=`), `(false, px)` for an upper one (`max-width`, `width<=`).
fn width_bound(query: &str) -> Option<(bool, i64)> {
    query.split(['(', ')']).find_map(|term| {
        if let Some(value) = term.strip_prefix("min-width:") {
            return Some((true, length_px(value)?));
        }
        if let Some(value) = term.strip_prefix("max-width:") {
            return Some((false, length_px(value)?));
        }
        let split = term.find(['<', '>'])?;
        let (left, right) = term.split_at(split);
        let lower = right.starts_with('>');
        let right = right.trim_start_matches(['<', '>', '=']);
        if left == "width" {
            Some((lower, length_px(right)?))
        } else if right == "width" || right.starts_with("width<") || right.starts_with("width>") {
            Some((!lower, length_px(left)?))
        } else {
            None
        }
    })
}

/// Sort key giving at-rule blocks a deterministic cascade.
///
/// Width queries come first, mobile-first (`min-width` ascending, then
/// `max-width` descending), then bare media types, then other media features,
/// then user preferences (`prefers-*`, `forced-colors`) so those win last.
#[must_use]
pub fn query_order(kind: AtRuleKind, query: &str) -> (u8, i64) {
    if matches!(kind, AtRuleKind::Media | AtRuleKind::Container)
        && let Some((lower, px)) = width_bound(query)
    {
        return if lower { (0, px) } else { (1, -px) };
    }
    if kind != AtRuleKind::Media {
        (3, 0)
    } else if query.contains("prefers-") || query.contains("forced-colors") {
        (4, 0)
    } else if query.contains('(') {
        (3, 0)
    } else {
        (2, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("print", Some("print"))]
    #[case("screen", Some("screen"))]
    #[case("all", Some("all"))]
    #[case("motionReduce", Some("(prefers-reduced-motion:reduce)"))]
    #[case("motion-reduce", Some("(prefers-reduced-motion:reduce)"))]
    #[case("motionSafe", Some("(prefers-reduced-motion:no-preference)"))]
    #[case("portrait", Some("(orientation:portrait)"))]
    #[case("landscape", Some("(orientation:landscape)"))]
    #[case("contrastMore", Some("(prefers-contrast:more)"))]
    #[case("contrastLess", Some("(prefers-contrast:less)"))]
    #[case("forcedColors", Some("(forced-colors:active)"))]
    #[case("speech", None)]
    #[case("hover", None)]
    fn test_media_shorthand_query(#[case] name: &str, #[case] expected: Option<&str>) {
        assert_eq!(media_shorthand_query(name), expected);
    }

    #[rstest]
    #[case("@media print", Some((AtRuleKind::Media, "print")))]
    #[case("@media(min-width: 1px)", Some((AtRuleKind::Media, "(min-width: 1px)")))]
    #[case(" @supports (display: grid) ", Some((AtRuleKind::Supports, "(display: grid)")))]
    #[case("@container sidebar (min-width: 1px)", Some((AtRuleKind::Container, "sidebar (min-width: 1px)")))]
    #[case("@media", None)]
    #[case("@media   ", None)]
    #[case("@mediaprint", None)]
    #[case("@layer reset", None)]
    #[case("media print", None)]
    fn test_split_at_rule_key(#[case] key: &str, #[case] expected: Option<(AtRuleKind, &str)>) {
        assert_eq!(split_at_rule_key(key), expected);
    }

    #[rstest]
    #[case("print", "print")]
    #[case("  print  ", "print")]
    #[case("(prefers-reduced-motion: reduce)", "(prefers-reduced-motion:reduce)")]
    #[case("( min-width : 768px )", "(min-width:768px)")]
    #[case(
        "(min-width: 768px) and (max-width: 1024px)",
        "(min-width:768px)and (max-width:1024px)"
    )]
    #[case("screen and (min-width: 500px)", "screen and (min-width:500px)")]
    #[case(
        "only screen and(min-width:768px)",
        "only screen and (min-width:768px)"
    )]
    #[case("not (display: grid)", "not (display:grid)")]
    #[case("not(display:grid)", "not (display:grid)")]
    #[case("(a) or(b)", "(a)or (b)")]
    #[case("sidebar (min-width: 400px)", "sidebar (min-width:400px)")]
    #[case(
        "(orientation: portrait), (hover: none)",
        "(orientation:portrait),(hover:none)"
    )]
    #[case("(width >= 600px)", "(width>=600px)")]
    #[case("(400px <= width <= 700px)", "(400px<=width<=700px)")]
    #[case("(min-width: calc(100px + 2em))", "(min-width:calc(100px + 2em))")]
    #[case("selector(a :hover)", "selector(a :hover)")]
    #[case("selector(:not(a))", "selector(:not(a))")]
    #[case("style(--label: \"a  b\")", "style(--label: \"a  b\")")]
    #[case("style(--label: 'it\\'s  x')", "style(--label: 'it\\'s  x')")]
    #[case("(not (color)) or (hover)", "(not (color))or (hover)")]
    #[case("", "")]
    fn test_normalize_query(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(normalize_query(query), expected);
    }

    #[rstest]
    #[case("(min-width:768px)", "(prefers-reduced-motion:reduce)", MediaCombination::Merged("(min-width:768px)and (prefers-reduced-motion:reduce)".to_string()))]
    #[case("(min-width:768px)", "print", MediaCombination::Merged("print and (min-width:768px)".to_string()))]
    #[case("print", "(prefers-reduced-motion:reduce)", MediaCombination::Merged("print and (prefers-reduced-motion:reduce)".to_string()))]
    #[case("(min-width:768px)", "screen and (min-width:100px)", MediaCombination::Merged("screen and (min-width:768px)and (min-width:100px)".to_string()))]
    #[case("(min-width:768px)", "only screen", MediaCombination::Merged("only screen and (min-width:768px)".to_string()))]
    #[case("(min-width:768px)", "not print", MediaCombination::Merged("screen and (min-width:768px)".to_string()))]
    #[case("(min-width:768px)", "not screen", MediaCombination::Merged("print and (min-width:768px)".to_string()))]
    #[case("(min-width:768px)", "not all", MediaCombination::Never)]
    #[case("(min-width:768px)", "not tv", MediaCombination::Nest)]
    #[case("(min-width:768px)", "not print and (color)", MediaCombination::Nest)]
    #[case("print", "screen", MediaCombination::Never)]
    #[case("print", "PRINT", MediaCombination::Merged("print".to_string()))]
    #[case("all", "print", MediaCombination::Merged("print".to_string()))]
    #[case("all", "all", MediaCombination::Merged("all".to_string()))]
    #[case("(min-width:768px)", "(orientation:portrait),(hover:none)", MediaCombination::Merged("(min-width:768px)and (orientation:portrait),(min-width:768px)and (hover:none)".to_string()))]
    #[case("(min-width:768px)", "print,screen", MediaCombination::Merged("print and (min-width:768px),screen and (min-width:768px)".to_string()))]
    #[case("print", "print,screen", MediaCombination::Merged("print".to_string()))]
    #[case("(min-width:768px)", "(a)or (b)", MediaCombination::Merged("(min-width:768px)and ((a)or (b))".to_string()))]
    #[case("print", "(a)or (b)", MediaCombination::Merged("print and ((a)or (b))".to_string()))]
    #[case("(min-width:768px)", "not (color)", MediaCombination::Merged("(min-width:768px)and (not (color))".to_string()))]
    #[case("print", "not (color)", MediaCombination::Merged("print and not (color)".to_string()))]
    #[case("(a)", "(b)", MediaCombination::Merged("(a)and (b)".to_string()))]
    #[case("(min-width:768px)", "print and speech", MediaCombination::Nest)]
    #[case("(min-width:768px)", "only", MediaCombination::Nest)]
    #[case("(min-width:768px)", "not", MediaCombination::Nest)]
    #[case("(min-width:768px)", "and", MediaCombination::Nest)]
    #[case("(min-width:768px)", "", MediaCombination::Nest)]
    fn test_combine_media_queries(
        #[case] outer: &str,
        #[case] inner: &str,
        #[case] expected: MediaCombination,
    ) {
        assert_eq!(combine_media_queries(outer, inner), expected);
    }

    #[rstest]
    #[case(AtRuleKind::Media, "(min-width:500px)", (0, 50_000))]
    #[case(AtRuleKind::Media, "(min-width:30em)", (0, 48_000))]
    #[case(AtRuleKind::Media, "(min-width:2rem)", (0, 3_200))]
    #[case(AtRuleKind::Media, "(width>=600px)", (0, 60_000))]
    #[case(AtRuleKind::Media, "(width>600px)", (0, 60_000))]
    #[case(AtRuleKind::Media, "(600px<=width)", (0, 60_000))]
    #[case(AtRuleKind::Media, "(400px<=width<=700px)", (0, 40_000))]
    #[case(AtRuleKind::Media, "(max-width:1000px)", (1, -100_000))]
    #[case(AtRuleKind::Media, "(width<=900px)", (1, -90_000))]
    #[case(AtRuleKind::Media, "(900px>=width)", (1, -90_000))]
    #[case(AtRuleKind::Media, "screen and (min-width:10px)", (0, 1_000))]
    #[case(AtRuleKind::Media, "(min-width:abc)", (3, 0))]
    #[case(AtRuleKind::Media, "(height>=10px)", (3, 0))]
    #[case(AtRuleKind::Media, "print", (2, 0))]
    #[case(AtRuleKind::Media, "(orientation:portrait)", (3, 0))]
    #[case(AtRuleKind::Media, "(prefers-reduced-motion:reduce)", (4, 0))]
    #[case(AtRuleKind::Media, "(forced-colors:active)", (4, 0))]
    #[case(AtRuleKind::Container, "(min-width:400px)", (0, 40_000))]
    #[case(AtRuleKind::Container, "sidebar (orientation:portrait)", (3, 0))]
    #[case(AtRuleKind::Supports, "(display:grid)", (3, 0))]
    fn test_query_order(
        #[case] kind: AtRuleKind,
        #[case] query: &str,
        #[case] expected: (u8, i64),
    ) {
        assert_eq!(query_order(kind, query), expected);
    }
}
