//! The theme of `devup.json` as the React Native output reads it: the names
//! of the themes in the order the runtime indexes them, the breakpoints, and
//! the value of each token and typography in every theme at each breakpoint,
//! resolved at build time.

use std::borrow::Cow;
use std::collections::BTreeMap;

use super::errors::Problem;

/// The values of a token at each breakpoint, `None` where it has none
pub type TokenLevels = Vec<Option<String>>;

/// The CSS properties of a typography, each with its value
pub type TypographyProperties = Vec<(String, String)>;

/// The breakpoints of the web, which `devup.json` keeps unless it sets its own
pub const DEFAULT_BREAKPOINTS: [u16; 6] = [0, 480, 768, 992, 1280, 1600];

/// The theme a file is compiled with. Built from `devup.json` by the
/// bindings, which read it with the web's theme parser
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// The window width, in density-independent pixels, each breakpoint
    /// starts at, as the web's media queries read them
    pub breakpoints: Vec<u16>,
}

impl Default for NativeTheme {
    fn default() -> Self {
        Self {
            names: Vec::new(),
            dark: None,
            tokens: BTreeMap::new(),
            typography: BTreeMap::new(),
            breakpoints: DEFAULT_BREAKPOINTS.to_vec(),
        }
    }
}

/// How many times a token's value may name another token
const MAX_TOKEN_DEPTH: usize = 8;

impl NativeTheme {
    /// How many style objects a value that differs by theme takes: one a
    /// theme, and one when the theme defines no theme at all
    pub(super) fn count(&self) -> usize {
        self.names.len().max(1)
    }

    /// How many breakpoints a responsive value can tell apart: at least one
    pub(super) fn levels(&self) -> usize {
        self.breakpoints.len().max(1)
    }

    /// The index of the theme `name`
    pub(super) fn index_of(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|theme| theme == name)
    }

    /// `text` with each `$token` replaced by its value at the breakpoint
    /// `level` in the theme at `index`, or `None` when that theme does not
    /// define one of them there
    pub(super) fn resolve<'t>(
        &self,
        text: &'t str,
        at: (usize, usize),
    ) -> Result<Option<Cow<'t, str>>, Problem> {
        if !text.contains('$') {
            return Ok(Some(Cow::Borrowed(text)));
        }
        let mut resolved = text.to_string();
        for _ in 0..MAX_TOKEN_DEPTH {
            match self.resolve_once(&resolved, at)? {
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
    fn resolve_once(&self, text: &str, at: (usize, usize)) -> Result<Option<String>, Problem> {
        let mut resolved = String::with_capacity(text.len());
        let mut rest = text;
        while let Some((before, token, after)) = next_token(rest) {
            resolved.push_str(before);
            match self.token(token, at)? {
                Some(value) => resolved.push_str(value),
                None => return Ok(None),
            }
            rest = after;
        }
        resolved.push_str(rest);
        Ok(Some(resolved))
    }

    /// The value of `token` (as written after `$`) at the breakpoint `level`
    /// in the theme at `index`: its value at the last breakpoint up to
    /// `level` that gives one, as the web's media queries cascade
    fn token(&self, token: &str, (index, level): (usize, usize)) -> Result<Option<&str>, Problem> {
        let key = token.replace('.', "-");
        let themes = self.tokens.get(&key).ok_or_else(|| {
            Problem::new(
                format!("devup.json defines no `${token}` token"),
                "add the token to the theme in devup.json, or write the value itself",
            )
        })?;
        Ok(themes
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|levels| at_level(levels, level)))
    }

    /// The properties of the typography `name` at the breakpoint `level`:
    /// each property as the last breakpoint up to `level` that gives it
    pub(super) fn typography(
        &self,
        name: &str,
        level: usize,
    ) -> Result<TypographyProperties, Problem> {
        let levels = self.typography.get(name).ok_or_else(|| {
            Problem::new(
                format!("devup.json defines no `{name}` typography"),
                "add the typography to the theme in devup.json",
            )
        })?;
        let mut properties: TypographyProperties = Vec::new();
        for entry in levels.iter().take(level + 1).flatten() {
            for (property, value) in entry {
                match properties.iter_mut().find(|(name, _)| name == property) {
                    Some(existing) => existing.1.clone_from(value),
                    None => properties.push((property.clone(), value.clone())),
                }
            }
        }
        Ok(properties)
    }

    /// How many breakpoints the tokens `text` names tell apart, through the
    /// tokens their values name: one when none is responsive
    pub(super) fn token_levels(&self, text: &str) -> usize {
        self.token_levels_within(text, MAX_TOKEN_DEPTH)
    }

    fn token_levels_within(&self, text: &str, depth: usize) -> usize {
        let mut levels = 1;
        let mut rest = text;
        while let Some((_, token, after)) = next_token(rest) {
            let all = self.tokens.get(&token.replace('.', "-"));
            for values in all.into_iter().flatten().flatten() {
                levels = levels.max(values.len());
                if depth > 0 {
                    for value in values.iter().flatten() {
                        levels = levels.max(self.token_levels_within(value, depth - 1));
                    }
                }
            }
            rest = after;
        }
        levels
    }

    /// How many breakpoints the typography `name` tells apart, through the
    /// tokens its values name
    pub(super) fn typography_levels(&self, name: &str) -> usize {
        self.typography.get(name).map_or(1, |levels| {
            levels
                .iter()
                .flatten()
                .flatten()
                .map(|(_, value)| self.token_levels(value))
                .fold(levels.len(), usize::max)
        })
    }
}

/// The value of `levels` at the breakpoint `level`: the last one up to it
fn at_level(levels: &TokenLevels, level: usize) -> Option<&str> {
    levels
        .iter()
        .take(level + 1)
        .rev()
        .find_map(Option::as_deref)
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
