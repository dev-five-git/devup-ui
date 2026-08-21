use std::collections::BTreeMap;
use std::sync::{LazyLock, RwLock};

#[derive(Default, Debug)]
struct ThemeTokenRegistry {
    length: BTreeMap<String, Vec<u8>>,
    shadow: BTreeMap<String, Vec<u8>>,
}

static TOKEN_REGISTRY: LazyLock<RwLock<ThemeTokenRegistry>> =
    LazyLock::new(|| RwLock::new(ThemeTokenRegistry::default()));

pub fn set_theme_token_levels(
    length: BTreeMap<String, Vec<u8>>,
    shadow: BTreeMap<String, Vec<u8>>,
) {
    if let Ok(mut registry) = TOKEN_REGISTRY.write() {
        registry.length = length;
        registry.shadow = shadow;
    }
}

/// Look up a `$token` in the length and shadow registries.
/// Returns the responsive breakpoint levels if the token is defined
/// with more than one level, regardless of which CSS property it's used on.
pub fn get_responsive_theme_token(value: &str) -> Option<Vec<u8>> {
    let token = value.strip_prefix('$')?;
    let registry = TOKEN_REGISTRY.read().ok()?;

    registry
        .length
        .get(token)
        .or_else(|| registry.shadow.get(token))
        .filter(|levels| levels.len() > 1)
        .cloned()
}

/// Returns `true` when the `$token` is defined with more than one responsive level.
///
/// Mirrors [`get_responsive_theme_token`] but without cloning the levels `Vec`;
/// use this at call sites that only need existence (`.is_some()`).
pub fn is_responsive_theme_token(value: &str) -> bool {
    // Same `?` shape as `get_responsive_theme_token` above: a poisoned registry
    // and a non-`$` value both fall out through the shared `unwrap_or(false)`
    // instead of each getting its own `return false;` statement.
    fn lookup(value: &str) -> Option<bool> {
        let token = value.strip_prefix('$')?;
        let registry = TOKEN_REGISTRY.read().ok()?;

        Some(
            registry
                .length
                .get(token)
                .or_else(|| registry.shadow.get(token))
                .is_some_and(|levels| levels.len() > 1),
        )
    }

    lookup(value).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_responsive_theme_token() {
        let mut length = BTreeMap::new();
        length.insert("containerX".to_string(), vec![0, 2]);
        let mut shadow = BTreeMap::new();
        shadow.insert("card".to_string(), vec![0, 3]);
        set_theme_token_levels(length, shadow);

        assert_eq!(get_responsive_theme_token("$containerX"), Some(vec![0, 2]));
        assert_eq!(get_responsive_theme_token("$card"), Some(vec![0, 3]));
        assert_eq!(get_responsive_theme_token("$unknown"), None);
        assert_eq!(get_responsive_theme_token("noprefix"), None);
    }

    #[test]
    fn test_is_responsive_theme_token() {
        let mut length = BTreeMap::new();
        length.insert("containerX".to_string(), vec![0, 2]);
        length.insert("single".to_string(), vec![0]);
        let mut shadow = BTreeMap::new();
        shadow.insert("card".to_string(), vec![0, 3]);
        set_theme_token_levels(length, shadow);

        assert!(is_responsive_theme_token("$containerX"));
        assert!(is_responsive_theme_token("$card"));
        // single-level tokens are not "responsive"
        assert!(!is_responsive_theme_token("$single"));
        assert!(!is_responsive_theme_token("$unknown"));
        assert!(!is_responsive_theme_token("noprefix"));
    }

    #[test]
    fn test_is_responsive_theme_token_without_prefix() {
        let value = std::hint::black_box("noprefix");

        assert!(!is_responsive_theme_token(value));
    }
}
