//! The priority `@stylexjs/babel-plugin` 0.19 gives a `stylex.when.*()`
//! condition, read from the `:where(...)` selector it compiles to.
//!
//! Taken from `getPseudoClassPriority` and `RELATIONAL_SELECTORS` of its
//! `property-priorities`. The priorities are scaled by 100 to stay integers,
//! and all sort before every other pseudo-class: the lowest of those is 40.

/// The pseudo-class priorities of `StyleX`, as written there: `:focusWithin`
/// and `:focusVisible` keep their camelCase spelling, so the kebab-case
/// pseudo-classes take the default of 40, as in `StyleX`
static PSEUDO_CLASS_PRIORITIES: phf::Map<&'static str, u16> = phf::phf_map! {
    ":is" => 40, ":where" => 40, ":not" => 40, ":has" => 45, ":dir" => 50,
    ":lang" => 51, ":first-child" => 52, ":first-of-type" => 53, ":last-child" => 54,
    ":last-of-type" => 55, ":only-child" => 56, ":only-of-type" => 57, ":nth-child" => 60,
    ":nth-last-child" => 61, ":nth-of-type" => 62, ":nth-last-of-type" => 63, ":empty" => 70,
    ":link" => 80, ":any-link" => 81, ":local-link" => 82, ":target-within" => 83,
    ":target" => 84, ":visited" => 85, ":enabled" => 91, ":disabled" => 92, ":required" => 93,
    ":optional" => 94, ":read-only" => 95, ":read-write" => 96, ":placeholder-shown" => 97,
    ":in-range" => 98, ":out-of-range" => 99, ":default" => 100, ":checked" => 101,
    ":indeterminate" => 101, ":blank" => 102, ":valid" => 103, ":invalid" => 104,
    ":user-invalid" => 105, ":autofill" => 110, ":picture-in-picture" => 120, ":modal" => 121,
    ":fullscreen" => 122, ":paused" => 123, ":playing" => 124, ":current" => 125,
    ":past" => 126, ":future" => 127, ":hover" => 130, ":focusWithin" => 140, ":focus" => 150,
    ":focusVisible" => 160, ":active" => 170,
};

/// The priority `StyleX` gives a pseudo-class it does not list
const DEFAULT_PSEUDO_PRIORITY: u16 = 40;

fn pseudo_priority(pseudo: &str) -> u16 {
    PSEUDO_CLASS_PRIORITIES
        .get(pseudo)
        .copied()
        .unwrap_or(DEFAULT_PSEUDO_PRIORITY)
}

/// The priority, scaled by 100, `StyleX` gives the condition `selector`, one
/// pseudo-class `:where(...)`; `None` for any other selector, as one that
/// follows the `:where(...)` with another pseudo-class
#[must_use]
pub fn relational_priority(selector: &str) -> Option<u16> {
    let inner = selector.strip_prefix(":where(")?.strip_suffix(')')?;
    Some(relational_shape(inner).unwrap_or(DEFAULT_PSEUDO_PRIORITY * 100))
}

/// The priority of the `:where(...)` contents `inner` when it is one of the
/// shapes `stylex.when.*()` gives
fn relational_shape(inner: &str) -> Option<u16> {
    let (base, pseudos) = match inner.strip_prefix(":has(") {
        Some(has) => has_shape(has.strip_suffix(')')?)?,
        None => sibling_shape(inner).or_else(|| ancestor_shape(inner))?,
    };
    let priority = pseudos.into_iter().map(pseudo_priority).max()?;
    Some(base * 100 + priority)
}

/// `.marker:pseudo` (`descendant`) or `~ .marker:pseudo` (`siblingAfter`)
fn has_shape(has: &str) -> Option<(u16, Vec<&str>)> {
    match has.strip_prefix('~') {
        Some(after) => Some((40, vec![marker_pseudo(single_space(after)?)?])),
        None => Some((15, vec![marker_pseudo(has)?])),
    }
}

/// `.marker:pseudo *` (`ancestor`)
fn ancestor_shape(inner: &str) -> Option<(u16, Vec<&str>)> {
    Some((10, vec![marker_pseudo(before_spaces(inner, "*")?)?]))
}

/// `.marker:pseudo ~ *` (`siblingBefore`), or that followed by
/// `, :has(~ .marker:pseudo)` (`anySibling`)
fn sibling_shape(inner: &str) -> Option<(u16, Vec<&str>)> {
    let Some((before, after)) = inner.split_once(',') else {
        return Some((30, vec![sibling_before(inner)?]));
    };
    let has = after
        .trim_start_matches(is_space)
        .strip_prefix(":has(~")?
        .strip_suffix(')')?;
    (after.starts_with(is_space)).then_some(())?;
    Some((
        20,
        vec![sibling_before(before)?, marker_pseudo(single_space(has)?)?],
    ))
}

fn sibling_before(text: &str) -> Option<&str> {
    marker_pseudo(before_spaces(before_spaces(text, "*")?, "~")?)
}

/// `text` without `suffix` and the whitespace, at least one, before it
fn before_spaces<'t>(text: &'t str, suffix: &str) -> Option<&'t str> {
    let rest = text.strip_suffix(suffix)?;
    let trimmed = rest.trim_end_matches(is_space);
    (trimmed.len() < rest.len()).then_some(trimmed)
}

/// `text` without the one whitespace character it starts with
fn single_space(text: &str) -> Option<&str> {
    let mut chars = text.chars();
    chars.next().filter(|c| is_space(*c))?;
    Some(chars.as_str())
}

const fn is_space(c: char) -> bool {
    c.is_whitespace()
}

/// The pseudo-class of `.marker:pseudo`, a class of word characters and
/// hyphens followed by one pseudo-class of letters and hyphens
fn marker_pseudo(text: &str) -> Option<&str> {
    let class = text.strip_prefix('.')?;
    let colon = class.find(':')?;
    let (name, pseudo) = class.split_at(colon);
    let pseudo_name = &pseudo[1..];
    (!name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && !pseudo_name.is_empty()
        && pseudo_name
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '-'))
    .then_some(pseudo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(":where(.x-default-marker:hover *)", Some(1130))]
    #[case(":where(.m:focus  *)", Some(1150))]
    #[case(":where(:has(.x-default-marker:focus))", Some(1650))]
    #[case(
        ":where(.x-default-marker:active ~ *, :has(~ .x-default-marker:hover))",
        Some(2170)
    )]
    #[case(
        ":where(.x-default-marker:hover ~ *, :has(~ .x-default-marker:active))",
        Some(2170)
    )]
    #[case(":where(.x-default-marker:hover ~ *)", Some(3130))]
    #[case(":where(:has(~ .x-default-marker:focus-visible))", Some(4040))]
    #[case(":where(:has(~ .x-default-marker[data-x]))", Some(4000))]
    #[case(":where(.x-default-marker:nth-child(2) *)", Some(4000))]
    #[case(":where(.x-default-marker:hover)", Some(4000))]
    #[case(":where(:has(~.m:hover))", Some(4000))]
    #[case(":where(:has(.m:hover)", Some(4000))]
    #[case(":where(.m:hover *):focus", None)]
    #[case(":where(.m:hover *):not(.x)", Some(4000))]
    #[case(":where(.m:hover ~ *,:has(~ .m:hover))", Some(4000))]
    #[case(":where(.m:hover ~ *, :has(~ .m:hover)", Some(4000))]
    #[case(":where(.m:hover ~ *, :is(.m))", Some(4000))]
    #[case(":where(.m:hover*)", Some(4000))]
    #[case(":where(.:hover *)", Some(4000))]
    #[case(":where(.m: *)", Some(4000))]
    #[case(":where(.m *)", Some(4000))]
    #[case(":where(m:hover *)", Some(4000))]
    #[case(":where(.m$:hover *)", Some(4000))]
    #[case(":where(.m:hover1 *)", Some(4000))]
    #[case(":where(:has(~ .m:hover), x)", Some(4000))]
    #[case(":hover", None)]
    fn test_relational_priority(#[case] selector: &str, #[case] priority: Option<u16>) {
        assert_eq!(relational_priority(selector), priority);
    }
}
