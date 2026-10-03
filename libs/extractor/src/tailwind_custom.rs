//! The utilities the project's Tailwind CSS defines with `@utility`: plain
//! ones (declarations and `@apply`) and functional ones (`@utility tab-*`
//! that read `--value()` and `--modifier()`)

use std::cell::Cell;

use css::tailwind_definitions::{UtilityItem, custom_utility, functional_utility, theme_value};

use crate::tailwind::{
    Declaration, ValueType, apply_utility, bracketed, decl, decode_arbitrary_value,
    is_positive_integer, spacing_scale, split_top_level, value_type,
};
use crate::tailwind_color::color_value;

/// How deep `@apply` goes through utilities that apply utilities
const MAX_APPLY_DEPTH: u8 = 8;

thread_local! {
    static APPLY_DEPTH: Cell<u8> = const { Cell::new(0) };
}

/// The declarations of the `@utility` named `name`, with the utilities it
/// applies written out; `None` when one of them is not a utility this build
/// compiles in full
pub fn custom_declarations(name: &str) -> Option<Vec<Declaration>> {
    let items = custom_utility(name)?;
    let depth = APPLY_DEPTH.with(|depth| {
        depth.set(depth.get() + 1);
        depth.get()
    });
    let declarations = (depth <= MAX_APPLY_DEPTH).then(|| expand(&items)).flatten();
    APPLY_DEPTH.with(|depth| depth.set(depth.get() - 1));
    declarations
}

fn expand(items: &[UtilityItem]) -> Option<Vec<Declaration>> {
    let mut declarations = Vec::new();
    for item in items {
        match item {
            UtilityItem::Declaration(property, value) => {
                declarations.push(decl(property.clone(), value.clone()));
            }
            UtilityItem::Apply(tokens) => {
                for token in tokens {
                    declarations.extend(apply_utility(token)?);
                }
            }
        }
    }
    Some(declarations)
}

/// What a candidate holds: the value after the root and its modifier
struct Candidate<'a> {
    value: &'a str,
    modifier: Option<&'a str>,
}

/// The type of an argument of `--value()` or `--modifier()`, applied to
/// `value`
fn resolve_argument(argument: &str, value: &str) -> Option<String> {
    let argument = argument.trim();
    if let Some(literal) = argument
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
        .or_else(|| {
            argument
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
        })
    {
        return (literal == value).then(|| literal.to_string());
    }
    if let Some(namespace) = argument
        .strip_prefix("--")
        .and_then(|rest| rest.strip_suffix("-*"))
    {
        return themed_value(namespace, value);
    }
    if let Some(kind) = bracketed(argument) {
        let inner = bracketed(value)?;
        return arbitrary_of_type(kind, &decode_arbitrary_value(inner));
    }
    let bare = match argument {
        "integer" => is_positive_integer(value),
        "number" => value
            .parse::<f64>()
            .is_ok_and(|number| number >= 0.0 && number.to_string() == value),
        "percentage" => value.strip_suffix('%').is_some_and(is_positive_integer),
        _ => false,
    };
    bare.then(|| value.to_string())
}

/// The value of `--<namespace>-<key>` that the project defines, or that the
/// defaults of the namespace give
fn themed_value(namespace: &str, key: &str) -> Option<String> {
    match namespace {
        "color" => color_value(key).map(std::borrow::Cow::into_owned),
        "spacing" => spacing_scale(key).map(std::borrow::Cow::into_owned),
        _ => theme_value(namespace, key),
    }
}

/// `value` when it is an arbitrary value of the data type `kind`
fn arbitrary_of_type(kind: &str, value: &str) -> Option<String> {
    let matches = match kind {
        "length" => value_type(None, value) == Some(ValueType::Length),
        "number" | "integer" => value_type(None, value) == Some(ValueType::Number),
        "percentage" => value
            .strip_suffix('%')
            .is_some_and(|n| n.parse::<f64>().is_ok()),
        "color" => value_type(None, value) == Some(ValueType::Color),
        "url" | "image" => value_type(None, value) == Some(ValueType::Image),
        _ => false,
    };
    matches.then(|| value.to_string())
}

/// The first argument of `function(...)` that `value` fits
fn resolve_function(arguments: &str, value: Option<&str>) -> Option<String> {
    let value = value?;
    split_top_level(arguments, ',')
        .into_iter()
        .find_map(|argument| resolve_argument(argument, value))
}

/// `declaration` with its `--value()` and `--modifier()` calls replaced:
/// `Err(())` when `--value()` has no match, `Ok(None)` when the declaration
/// drops out for want of a modifier
fn substitute(text: &str, candidate: &Candidate<'_>) -> Result<Option<String>, ()> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("--") {
        let after = &rest[start..];
        let (name, is_value) = if after.starts_with("--value(") {
            ("--value(", true)
        } else if after.starts_with("--modifier(") {
            ("--modifier(", false)
        } else {
            out.push_str(&rest[..start + 2]);
            rest = &rest[start + 2..];
            continue;
        };
        out.push_str(&rest[..start]);
        let open = start + name.len();
        let close = matching_paren(&rest[open..]).ok_or(())? + open;
        let arguments = &rest[open..close];
        let resolved = if is_value {
            resolve_function(arguments, Some(candidate.value))
        } else {
            resolve_function(arguments, candidate.modifier)
        };
        match (resolved, is_value) {
            (Some(resolved), _) => out.push_str(&resolved),
            (None, true) => return Err(()),
            (None, false) => return Ok(None),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    Ok(Some(out))
}

/// The index of the `)` that closes a `(` just before `text`
fn matching_paren(text: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (index, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether `text` still calls a function of `@utility` that this build does
/// not run (`--spacing()`, `--alpha()` ...)
fn calls_unknown_function(text: &str) -> bool {
    text.match_indices("--").any(|(index, _)| {
        let name = &text[index + 2..];
        let end = name
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(name.len());
        name[end..].starts_with('(') && end > 0
    })
}

/// The declarations of the functional `@utility` that `name` is a candidate
/// of (`tab-4` for `tab-*`)
pub fn functional_declarations(name: &str) -> Option<Vec<Declaration>> {
    name.match_indices('-').find_map(|(index, _)| {
        let declarations = functional_utility(&name[..index])?;
        let rest = &name[index + 1..];
        let parts = split_top_level(rest, '/');
        let candidate = match parts.as_slice() {
            [value] => Candidate {
                value,
                modifier: None,
            },
            [value, modifier] => Candidate {
                value,
                modifier: Some(modifier),
            },
            _ => return None,
        };
        let mut resolved = Vec::new();
        for (property, value) in declarations {
            let Ok(value) = substitute(&value, &candidate) else {
                return None;
            };
            let Some(value) = value else {
                continue;
            };
            if calls_unknown_function(&value) {
                return None;
            }
            resolved.push(decl(property, value));
        }
        (!resolved.is_empty()).then_some(resolved)
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use css::tailwind_definitions::set_tailwind_css;
    use serial_test::serial;

    use crate::tailwind::declarations_of;

    fn written(class: &str) -> Vec<(String, String)> {
        declarations_of(class).unwrap_or_else(|| panic!("{class} stays as written"))
    }

    fn pairs(declarations: &[(&str, &str)]) -> Vec<(String, String)> {
        declarations
            .iter()
            .map(|(property, value)| (property.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    #[serial]
    fn apply_writes_out_the_utilities_it_applies() {
        set_tailwind_css(
            "@utility btn { @apply px-4 py-2!; border-radius: 4px; } @utility btn-lg { @apply btn text-lg; } @utility loop-a { @apply loop-b; } @utility loop-b { @apply loop-a; } @utility hovered { @apply hover:p-4; } @utility unknown { @apply nothing-here; } @utility children { @apply space-x-4; }",
        );
        assert_eq!(
            written("btn"),
            pairs(&[
                ("padding-inline", "1rem"),
                ("padding-block", "0.5rem !important"),
                ("border-radius", "4px"),
            ])
        );
        let lg = written("btn-lg");
        assert_eq!(lg[0], ("padding-inline".to_string(), "1rem".to_string()));
        assert_eq!(lg[3].0, "font-size");
        for class in ["loop-a", "hovered", "unknown", "children"] {
            assert_eq!(declarations_of(class), None, "{class}");
        }
        set_tailwind_css("");
    }

    #[test]
    #[serial]
    fn functional_utilities_read_their_value_and_modifier() {
        set_tailwind_css(
            "@theme { --tab-size-github: 8; --color-brand: #0af; --spacing-gutter: 3rem; } @utility tab-* { tab-size: --value(--tab-size-*, integer, [integer]); } @utility pct-* { width: --value(percentage); } @utility lit-* { display: --value('inline-flex', 'grid'); } @utility num-* { line-height: --value(number); } @utility bg-glow-* { background: --value(--color-*, [color]); box-shadow: 0 0 8px --modifier([percentage], number); } @utility gap-* { gap: --value(--spacing-*); } @utility len-* { width: --value([length]); } @utility pc-* { height: --value([percentage]); } @utility many-* { a: --value([url], [number], [unknown]); } @utility q-* { content: \"--value(x\"; } @utility space-* { gap: --spacing(4); } @utility bad-* { width: --value(unknown-type); } @utility mod-* { width: --value(integer); color: --modifier('red'); }",
        );
        assert_eq!(written("tab-4"), pairs(&[("tab-size", "4")]));
        assert_eq!(written("tab-github"), pairs(&[("tab-size", "8")]));
        assert_eq!(written("tab-[12]"), pairs(&[("tab-size", "12")]));
        assert_eq!(declarations_of("tab-1.5"), None);
        assert_eq!(declarations_of("tab-nothing"), None);
        assert_eq!(declarations_of("tab-"), None);
        assert_eq!(declarations_of("tab-4/2/3"), None);
        assert_eq!(written("pct-50%"), pairs(&[("width", "50%")]));
        assert_eq!(declarations_of("pct-50"), None);
        assert_eq!(written("lit-grid"), pairs(&[("display", "grid")]));
        assert_eq!(
            written("lit-inline-flex"),
            pairs(&[("display", "inline-flex")])
        );
        assert_eq!(declarations_of("lit-block"), None);
        assert_eq!(written("num-1.5"), pairs(&[("line-height", "1.5")]));
        assert_eq!(written("bg-glow-brand"), pairs(&[("background", "#0af")]));
        assert_eq!(
            written("bg-glow-red-500/50"),
            pairs(&[
                ("background", "oklch(63.7% 0.237 25.331)"),
                ("box-shadow", "0 0 8px 50"),
            ])
        );
        assert_eq!(written("bg-glow-[#f00]"), pairs(&[("background", "#f00")]));
        assert_eq!(written("gap-gutter"), pairs(&[("gap", "3rem")]));
        assert_eq!(written("gap-4"), pairs(&[("gap", "1rem")]));
        assert_eq!(written("len-[3px]"), pairs(&[("width", "3px")]));
        assert_eq!(declarations_of("len-3px"), None);
        assert_eq!(written("many-[1]"), pairs(&[("a", "1")]));
        assert_eq!(declarations_of("many-[x]"), None);
        for class in ["space-4", "bad-4", "q-1"] {
            assert_eq!(declarations_of(class), None, "{class}");
        }
        assert_eq!(written("mod-4"), pairs(&[("width", "4")]));
        assert_eq!(
            written("mod-4/red"),
            pairs(&[("width", "4"), ("color", "red")])
        );
        assert_eq!(written("pc-[50%]"), pairs(&[("height", "50%")]));
        set_tailwind_css("");
    }
}
