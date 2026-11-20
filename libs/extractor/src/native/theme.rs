//! The theme of `devup.json` as the React Native output reads it: the names
//! of the themes in the order the runtime indexes them, and the value of each
//! token and typography in every theme, resolved at build time.

use std::borrow::Cow;
use std::collections::BTreeMap;

use super::NativeTarget;
use super::errors::{Problem, RuntimeFeature};

/// The values of a token at each breakpoint, `None` where it has none
pub type TokenLevels = Vec<Option<String>>;

/// The CSS properties of a typography, each with its value
pub type TypographyProperties = Vec<(String, String)>;

/// The theme a file is compiled with. Built from `devup.json` by the
/// bindings, which read it with the web's theme parser
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeTheme {
    /// The theme names, the default first: the runtime picks a style object by
    /// the index of a name here
    pub names: Vec<String>,
    /// The index of the theme the system's dark appearance shows, if any
    pub dark: Option<usize>,
    /// Each token by its CSS variable name (`primary`, `gray-100`), with its
    /// values in each theme, in the order of `names`; `None` in a theme that
    /// does not define it
    pub tokens: BTreeMap<String, Vec<Option<TokenLevels>>>,
    /// Each typography by name, with its properties at each breakpoint
    pub typography: BTreeMap<String, Vec<Option<TypographyProperties>>>,
}

/// How many times a token's value may name another token
const MAX_TOKEN_DEPTH: usize = 8;

impl NativeTheme {
    /// How many style objects a value that differs by theme takes: one a
    /// theme, and one when the theme defines no theme at all
    pub(super) fn count(&self) -> usize {
        self.names.len().max(1)
    }

    /// The index of the theme `name`
    pub(super) fn index_of(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|theme| theme == name)
    }

    /// `text` with each `$token` replaced by its value in the theme at
    /// `index`, or `None` when that theme does not define one of them
    pub(super) fn resolve<'t>(
        &self,
        text: &'t str,
        index: usize,
        target: &NativeTarget,
    ) -> Result<Option<Cow<'t, str>>, Problem> {
        if !text.contains('$') {
            return Ok(Some(Cow::Borrowed(text)));
        }
        let mut resolved = text.to_string();
        for _ in 0..MAX_TOKEN_DEPTH {
            match self.resolve_once(&resolved, index, target)? {
                Some(next) if next == resolved => return Ok(Some(Cow::Owned(next))),
                Some(next) => resolved = next,
                None => return Ok(None),
            }
        }
        Err(Problem::new(
            format!("the tokens in `{text}` name each other without end"),
            "give one of the tokens a value in devup.json",
        ))
    }

    /// `text` with each `$token` replaced once
    fn resolve_once(
        &self,
        text: &str,
        index: usize,
        target: &NativeTarget,
    ) -> Result<Option<String>, Problem> {
        let mut resolved = String::with_capacity(text.len());
        let mut rest = text;
        while let Some((before, token, after)) = next_token(rest) {
            resolved.push_str(before);
            match self.token(token, index, target)? {
                Some(value) => resolved.push_str(value),
                None => return Ok(None),
            }
            rest = after;
        }
        resolved.push_str(rest);
        Ok(Some(resolved))
    }

    /// The value of `token` (as written after `$`) in the theme at `index`
    fn token(
        &self,
        token: &str,
        index: usize,
        target: &NativeTarget,
    ) -> Result<Option<&str>, Problem> {
        let key = token.replace('.', "-");
        let themes = self.tokens.get(&key).ok_or_else(|| {
            Problem::new(
                format!("devup.json defines no `${token}` token"),
                "add the token to the theme in devup.json, or write the value itself",
            )
        })?;
        if themes.iter().flatten().any(is_responsive) {
            return Err(responsive_token(token, target));
        }
        Ok(themes
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|levels| levels.first())
            .and_then(Option::as_deref))
    }

    /// The properties of the typography `name`
    pub(super) fn typography(
        &self,
        name: &str,
        target: &NativeTarget,
    ) -> Result<&TypographyProperties, Problem> {
        let levels = self.typography.get(name).ok_or_else(|| {
            Problem::new(
                format!("devup.json defines no `{name}` typography"),
                "add the typography to the theme in devup.json",
            )
        })?;
        match levels.split_first() {
            Some((Some(properties), rest)) if rest.iter().all(Option::is_none) => Ok(properties),
            _ => Err(responsive_token(name, target)),
        }
    }
}

/// Whether a token has a value at a breakpoint but the first, which the
/// window width chooses
fn is_responsive(levels: &TokenLevels) -> bool {
    !matches!(levels.split_first(), Some((Some(_), rest)) if rest.iter().all(Option::is_none))
}

/// Why a token with a value at each breakpoint cannot compile yet
fn responsive_token(name: &str, target: &NativeTarget) -> Problem {
    let problem = RuntimeFeature::Responsive.problem(target);
    Problem::new(
        format!(
            "`{name}` has a value at each breakpoint: {}",
            problem.reason
        ),
        "give it one value in devup.json, or write the value itself",
    )
}

/// The text before the first `$token` of `text`, the token's name and the
/// text after it. A token is `$`, a word character, then word characters,
/// dots and dashes, as on the web
pub(super) fn next_token(text: &str) -> Option<(&str, &str, &str)> {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(offset) = text[from..].find('$') {
        let start = from + offset;
        let name_start = start + 1;
        if bytes.get(name_start).is_some_and(|b| is_word(*b)) {
            let end = bytes[name_start..]
                .iter()
                .position(|b| !is_word(*b) && !matches!(b, b'.' | b'-'))
                .map_or(bytes.len(), |length| name_start + length);
            return Some((&text[..start], &text[name_start..end], &text[end..]));
        }
        from = name_start;
    }
    None
}

const fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The theme a `_theme*` prop names: `_themeDark` is `dark`, `_themeHighContrast`
/// is `highContrast`, as on the web. `prop` starts with `_theme`
pub(super) fn selector_theme(prop: &str) -> String {
    let rest = &prop["_theme".len()..];
    css::utils::to_camel_case(&css::utils::to_kebab_case(rest)).into_owned()
}
