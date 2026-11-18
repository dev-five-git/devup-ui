//! `StyleX`'s `styleResolution` read at build time: under `application-order`
//! and `legacy-expand-shorthands`, a `stylex.create()` key expands as
//! `@stylexjs/babel-plugin` expands it, into the keys it sets and the keys it
//! resets to `null`. `stylex.props()` then merges those keys as `StyleX` does:
//! a later key wins and a `null` key removes the earlier one.
//! `property-specificity` (the default) expands nothing: the stylesheet writes
//! a longhand after every shorthand covering it.

use css::style_resolution::StyleResolution;

#[path = "stylex_resolution_tables.rs"]
mod tables;

/// How a key's value spreads over the keys it sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Split {
    /// Every key gets the whole value
    Whole,
    /// `a b` over two keys, the second defaulting to the first
    Pair,
    /// `top right bottom left` over four keys, as `margin` reads them
    Box,
    /// `containIntrinsicSize`: an `auto` joins the size after it
    ContainIntrinsicSize,
    /// `listStyle`: its parts sorted into type, position and image
    ListStyle,
    /// A key `StyleX` rejects under the policy
    Unsupported(&'static str),
}

/// What one `stylex.create()` key expands to
#[derive(Debug)]
pub(crate) struct Expansion {
    split: Split,
    targets: &'static [&'static str],
    resets: &'static [&'static str],
}

/// A key a value expands to, with its value; `None` resets it
pub(crate) type ExpandedKey = (&'static str, Option<String>);

impl Expansion {
    pub(crate) const fn new(
        split: Split,
        targets: &'static [&'static str],
        resets: &'static [&'static str],
    ) -> Self {
        Self {
            split,
            targets,
            resets,
        }
    }

    /// The keys `value` sets and resets, in the order `StyleX` gives them
    pub(crate) fn apply(&self, value: Option<&str>) -> Result<Vec<ExpandedKey>, String> {
        let values = spread(self.split, value, self.targets.len())?;
        Ok(self
            .targets
            .iter()
            .copied()
            .zip(values)
            .chain(self.resets.iter().map(|reset| (*reset, None)))
            .collect())
    }
}

/// What `key` expands to under `policy`; `None` when it stays as written
pub(crate) fn expansion(policy: StyleResolution, key: &str) -> Option<&'static Expansion> {
    match policy {
        StyleResolution::PropertySpecificity => None,
        StyleResolution::ApplicationOrder => tables::APPLICATION_ORDER.get(key),
        StyleResolution::LegacyExpandShorthands => tables::LEGACY_EXPAND_SHORTHANDS.get(key),
    }
}

/// The values of the `keys` keys `split` spreads `value` over; a `null`
/// value resets every one of them
fn spread(split: Split, value: Option<&str>, keys: usize) -> Result<Vec<Option<String>>, String> {
    let Some(value) = value else {
        return Ok(vec![None; keys]);
    };
    match split {
        Split::Whole => Ok(vec![Some(value.to_string()); keys]),
        Split::Pair => Ok(sides(split_value(value), &[0, 0])),
        Split::Box => Ok(sides(split_value(value), &[0, 0, 0, 1])),
        Split::ContainIntrinsicSize => Ok(sides(join_auto(split_value(value)), &[0, 0])),
        Split::ListStyle => list_style(value),
        Split::Unsupported(message) => Err(message.to_string()),
    }
}

/// `parts` over sides whose missing part repeats the part `defaults` names
fn sides(parts: Vec<String>, defaults: &[usize]) -> Vec<Option<String>> {
    let mut sides: Vec<String> = Vec::with_capacity(defaults.len());
    let mut parts = parts.into_iter();
    for &default in defaults {
        let side = parts
            .next()
            .or_else(|| sides.get(default).cloned())
            .unwrap_or_default();
        sides.push(side);
    }
    sides.into_iter().map(Some).collect()
}

/// `auto 10px` stays one size, as `containIntrinsicSize` reads it
fn join_auto(parts: Vec<String>) -> Vec<String> {
    let mut joined: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts {
        match joined.last_mut() {
            Some(last) if last == "auto" => *last = format!("auto {part}"),
            _ => joined.push(part),
        }
    }
    joined
}

/// The parts of a value as `StyleX` splits it: at spaces, commas and slashes
/// outside functions and strings, a final `!important` applying to each part.
/// A value without parts is one empty part
pub(crate) fn split_value(value: &str) -> Vec<String> {
    let mut parts = vec![];
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for c in value.chars() {
        match quote {
            Some(open) => {
                current.push(c);
                quote = (c != open).then_some(open);
            }
            None if depth == 0 && (c.is_whitespace() || c == ',' || c == '/') => {
                flush(&mut parts, &mut current);
            }
            None => {
                depth = nesting(depth, c);
                quote = matches!(c, '"' | '\'').then_some(c);
                current.push(c);
            }
        }
    }
    flush(&mut parts, &mut current);
    important(parts)
}

/// The function depth after `c`
const fn nesting(depth: usize, c: char) -> usize {
    match c {
        '(' => depth + 1,
        ')' => depth.saturating_sub(1),
        _ => depth,
    }
}

fn flush(parts: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        parts.push(std::mem::take(current));
    }
}

/// A trailing `!important` marks every part before it
fn important(mut parts: Vec<String>) -> Vec<String> {
    if parts.len() > 1 && parts[parts.len() - 1].eq_ignore_ascii_case("!important") {
        parts.pop();
        return parts
            .into_iter()
            .map(|part| format!("{part} !important"))
            .collect();
    }
    if parts.is_empty() {
        parts.push(String::new());
    }
    parts
}

/// Values every `listStyle` longhand takes alike
const LIST_STYLE_GLOBALS: [&str; 4] = ["inherit", "initial", "revert", "unset"];

/// `listStyle` sorted into its type, position and image, as `StyleX` sorts it
fn list_style(value: &str) -> Result<Vec<Option<String>>, String> {
    let parts = split_value(value);
    if let [part] = parts.as_slice()
        && LIST_STYLE_GLOBALS.contains(&part.as_str())
    {
        return Ok(vec![Some(part.clone()); 3]);
    }
    let invalid = || format!("invalid \"listStyle\" value of {value:?}");
    let mut sorted = ListStyle::default();
    let mut remaining = vec![];
    for part in parts {
        if LIST_STYLE_GLOBALS.contains(&part.as_str()) || part.contains("var(--") {
            return Err(invalid());
        }
        if part == "inside" || part == "outside" {
            sorted.position = Some(take(sorted.position, part).ok_or_else(invalid)?);
        } else if part != "none" && is_list_style_type(&part) {
            sorted.kind = Some(take(sorted.kind, part).ok_or_else(invalid)?);
        } else {
            remaining.push(part);
        }
    }
    for part in remaining {
        if part == "none" && sorted.kind.is_none() {
            sorted.kind = Some(part);
        } else {
            sorted.image = Some(take(sorted.image, part).ok_or_else(invalid)?);
        }
    }
    Ok(vec![sorted.kind, sorted.position, sorted.image])
}

/// The longhands `listStyle` sorts its parts into
#[derive(Default)]
struct ListStyle {
    kind: Option<String>,
    position: Option<String>,
    image: Option<String>,
}

/// `part` for a longhand not set yet; `None` when it already is
fn take(set: Option<String>, part: String) -> Option<String> {
    set.is_none().then_some(part)
}

/// A keyword or a quoted string, as `StyleX` reads a `list-style-type`
fn is_list_style_type(part: &str) -> bool {
    let quoted = |quote: char| part.len() >= 2 && part.starts_with(quote) && part.ends_with(quote);
    let keyword = !part.is_empty()
        && part
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-');
    keyword || quoted('"') || quoted('\'')
}

#[cfg(test)]
#[path = "stylex_resolution_tests.rs"]
mod tests;
