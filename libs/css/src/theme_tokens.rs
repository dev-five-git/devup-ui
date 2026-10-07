use std::collections::BTreeMap;
use std::sync::{LazyLock, RwLock};

#[derive(Default, Debug)]
pub(crate) struct ThemeTokenRegistry {
    length: BTreeMap<String, Vec<u8>>,
    shadow: BTreeMap<String, Vec<u8>>,
    typography: Vec<String>,
}

static TOKEN_REGISTRY: LazyLock<RwLock<ThemeTokenRegistry>> =
    LazyLock::new(|| RwLock::new(ThemeTokenRegistry::default()));

pub(crate) fn replace_registry(registry: ThemeTokenRegistry) -> ThemeTokenRegistry {
    let previous = std::mem::replace(
        &mut *TOKEN_REGISTRY
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        registry,
    );
    TOKEN_REGISTRY.clear_poison();
    previous
}

/// Clear all responsive tokens and typography names at a quiescent test boundary.
pub fn reset_theme_tokens() {
    replace_registry(ThemeTokenRegistry::default());
}

pub fn set_theme_token_levels(
    length: BTreeMap<String, Vec<u8>>,
    shadow: BTreeMap<String, Vec<u8>>,
) {
    if let Ok(mut registry) = TOKEN_REGISTRY.write() {
        registry.length = length;
        registry.shadow = shadow;
    }
}

pub fn set_typography_keys(keys: Vec<String>) {
    if let Ok(mut registry) = TOKEN_REGISTRY.write() {
        registry.typography = keys;
    }
}

/// Typography names defined by the registered theme, so a dynamic
/// `typography` value under a selector can resolve to one class per name.
pub fn get_typography_keys() -> Vec<String> {
    TOKEN_REGISTRY
        .read()
        .map(|registry| registry.typography.clone())
        .unwrap_or_default()
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
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_responsive_theme_token() {
        let _state = crate::test_state::TestStateGuard::new();
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
    #[serial]
    fn test_is_responsive_theme_token() {
        let _state = crate::test_state::TestStateGuard::new();
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
    #[serial]
    fn test_typography_keys() {
        let _state = crate::test_state::TestStateGuard::new();
        set_typography_keys(vec!["body".to_string(), "title".to_string()]);
        assert_eq!(get_typography_keys(), vec!["body", "title"]);
        set_typography_keys(vec![]);
        assert_eq!(get_typography_keys(), Vec::<String>::new());
    }

    #[test]
    fn test_is_responsive_theme_token_without_prefix() {
        let value = std::hint::black_box("noprefix");

        assert!(!is_responsive_theme_token(value));
    }

    #[test]
    #[serial]
    fn scope_restores_registries_when_private_locks_are_poisoned() {
        let _state = crate::test_state::TestStateGuard::new();
        set_typography_keys(vec!["outer".into()]);
        crate::set_custom_shorthands(BTreeMap::from([("alias".into(), vec!["left".into()])]));
        let nested = crate::test_state::TestStateGuard::new();
        for poison_tokens in [false, true] {
            let result = std::panic::catch_unwind(|| {
                if poison_tokens {
                    let _lock = TOKEN_REGISTRY
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    panic!("poison tokens");
                }
                let _lock = crate::CUSTOM_SHORTHANDS
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                panic!("poison shorthands");
            });
            assert!(result.is_err());
        }
        crate::test_state::reset_state_for_testing();
        assert!(!crate::CUSTOM_SHORTHANDS.is_poisoned());
        assert!(!TOKEN_REGISTRY.is_poisoned());
        assert_eq!(crate::get_custom_shorthand_names(), Vec::<String>::new());
        assert_eq!(get_typography_keys(), Vec::<String>::new());
        drop(nested);
        assert_eq!(get_typography_keys(), vec!["outer"]);
        assert_eq!(
            crate::disassemble_property("alias").collect::<Vec<_>>(),
            vec!["left"]
        );
    }
}
