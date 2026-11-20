//! The registered theme as the React Native output reads it: every token and
//! typography resolved for each theme, from the theme the web parses.

use std::collections::{BTreeMap, BTreeSet};

use extractor::{NativeTheme, TokenLevels, TypographyProperties};
use sheet::theme::{ColorScheme, Theme, TokenValues, Typographies, default_variant_key};

/// `theme` for React Native, every token resolved for each theme.
///
/// The themes are the color variants, the default first and the rest by name,
/// as the web's `data-theme` names them; with no colors, the length and shadow
/// variants. A token a theme does not define takes the value of the default
/// variant of its kind, as the web's `:root` gives it
#[must_use]
pub fn native_theme(theme: &Theme) -> NativeTheme {
    let names = theme_names(theme);
    let mut tokens = BTreeMap::new();
    for key in theme
        .colors
        .values()
        .flat_map(|colors| colors.css_entries().map(|(key, _)| key))
    {
        tokens.entry(key.clone()).or_insert_with(|| {
            per_theme(&names, &theme.colors, |colors| {
                colors.get(key).map(|value| vec![Some(value.clone())])
            })
        });
    }
    for themes in [&theme.length, &theme.shadows] {
        for name in themes.values().flat_map(BTreeMap::keys) {
            tokens.entry(name.replace('.', "-")).or_insert_with(|| {
                per_theme(&names, themes, |values| values.get(name).map(levels))
            });
        }
    }
    let dark = dark_theme(theme, &names);
    NativeTheme {
        names,
        dark,
        tokens,
        typography: theme
            .typography
            .iter()
            .map(|(name, typography)| (name.clone(), typography_levels(typography)))
            .collect(),
    }
}

/// The theme names, the default first
fn theme_names(theme: &Theme) -> Vec<String> {
    let names: BTreeSet<&str> = if theme.colors.is_empty() {
        theme
            .length
            .keys()
            .chain(theme.shadows.keys())
            .map(String::as_str)
            .collect()
    } else {
        theme.colors.keys().map(String::as_str).collect()
    };
    // The variant the page shows without a `data-theme`, as the web picks it
    let default = ["default", "light"]
        .into_iter()
        .find(|name| names.contains(name))
        .or_else(|| names.first().copied());
    default
        .into_iter()
        .chain(names.iter().copied().filter(|name| Some(*name) != default))
        .map(str::to_string)
        .collect()
}

/// A token's value in each of `names`, read from its variant of `themes` by
/// `read`, else from the default variant
fn per_theme<T>(
    names: &[String],
    themes: &BTreeMap<String, T>,
    read: impl Fn(&T) -> Option<TokenLevels>,
) -> Vec<Option<TokenLevels>> {
    let fallback = default_variant_key(themes)
        .and_then(|default| themes.get(default))
        .and_then(&read);
    names
        .iter()
        .map(|name| {
            themes
                .get(name)
                .and_then(&read)
                .or_else(|| fallback.clone())
        })
        .collect()
}

fn levels(values: &TokenValues) -> TokenLevels {
    values.0.clone()
}

/// The theme the system's dark appearance shows: `dark`, as on the web, when
/// it renders dark, else the first theme `colorScheme` makes dark
fn dark_theme(theme: &Theme, names: &[String]) -> Option<usize> {
    let is_dark = |name: &String| theme.color_scheme_of(name) == ColorScheme::Dark;
    names
        .iter()
        .position(|name| name == "dark" && is_dark(name))
        .or_else(|| names.iter().position(is_dark))
}

/// The properties of a typography at each breakpoint
fn typography_levels(typography: &Typographies) -> Vec<Option<TypographyProperties>> {
    typography
        .0
        .iter()
        .map(|entry| {
            entry.as_ref().map(|entry| {
                entry
                    .properties()
                    .into_iter()
                    .filter_map(|(property, value)| {
                        value
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(|value| (property.to_string(), value.to_string()))
                    })
                    .collect()
            })
        })
        .collect()
}
