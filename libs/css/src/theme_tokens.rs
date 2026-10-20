use std::collections::{BTreeMap, BTreeSet};
use std::sync::{LazyLock, RwLock};

#[derive(Default, Debug)]
pub(crate) struct ThemeTokenRegistry {
    length: BTreeMap<String, Vec<u8>>,
    shadow: BTreeMap<String, Vec<u8>>,
    first_length: BTreeMap<String, String>,
    first_shadow: BTreeMap<String, String>,
    typography: Vec<String>,
    colors: BTreeSet<String>,
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
        registry.first_length.clear();
        registry.first_shadow.clear();
    }
}

/// Register effective default literals separately from responsive variable levels.
pub fn set_theme_token_values(length: BTreeMap<String, String>, shadow: BTreeMap<String, String>) {
    if let Ok(mut registry) = TOKEN_REGISTRY.write() {
        let normalize = |values: BTreeMap<String, String>| {
            values
                .into_iter()
                .map(|(token, value)| {
                    (
                        token,
                        crate::optimize_value::optimize_value(&value).into_owned(),
                    )
                })
                .collect()
        };
        registry.first_length = normalize(length);
        registry.first_shadow = normalize(shadow);
    }
}

/// Resolve the first default literal in the property's token namespace.
pub fn get_first_theme_token_value(property: &str, value: &str) -> Option<String> {
    let token = value.strip_prefix('$')?;
    let registry = TOKEN_REGISTRY.read().ok()?;
    let values = if property == "box-shadow" {
        &registry.first_shadow
    } else {
        &registry.first_length
    };
    values.get(token).cloned()
}

pub fn set_typography_keys(keys: Vec<String>) {
    if let Ok(mut registry) = TOKEN_REGISTRY.write() {
        registry.typography = keys;
    }
}

/// Registers the names of the theme's colors, which Tailwind classes such as
/// `bg-primary` can use
pub fn set_color_tokens(names: BTreeSet<String>) {
    if let Ok(mut registry) = TOKEN_REGISTRY.write() {
        registry.colors = names;
    }
}

/// Whether the theme defines the color `name`, a CSS variable of that name
pub fn is_color_token(name: &str) -> bool {
    TOKEN_REGISTRY
        .read()
        .is_ok_and(|registry| registry.colors.contains(name))
}

/// Whether the theme defines the shadow `name`, a CSS variable of that name
pub fn is_shadow_token(name: &str) -> bool {
    TOKEN_REGISTRY
        .read()
        .is_ok_and(|registry| registry.shadow.contains_key(name))
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
    fn theme_token_values_resolve_in_the_property_namespace() {
        // Given
        set_theme_token_values(
            BTreeMap::from([("shared".into(), "8px".into())]),
            BTreeMap::from([("shared".into(), "0 1px 2px black".into())]),
        );
        // When / Then
        assert_eq!(
            get_first_theme_token_value("width", "$shared"),
            Some("8px".into())
        );
        assert_eq!(
            get_first_theme_token_value("box-shadow", "$shared"),
            Some("0 1px 2px black".into())
        );
        assert_eq!(get_first_theme_token_value("width", "$missing"), None);
        assert_eq!(get_first_theme_token_value("width", "shared"), None);
        set_theme_token_levels(BTreeMap::new(), BTreeMap::new());
    }

    #[test]
    #[serial]
    fn theme_token_values_normalize_zero_literals() {
        // Given
        let zero = BTreeMap::from([("zero".into(), "0px".into())]);
        // When
        set_theme_token_values(zero.clone(), zero);
        // Then
        assert_eq!(
            get_first_theme_token_value("width", "$zero"),
            Some("0".into())
        );
        assert_eq!(
            get_first_theme_token_value("box-shadow", "$zero"),
            Some("0".into())
        );
        set_theme_token_levels(BTreeMap::new(), BTreeMap::new());
    }

    #[test]
    #[serial]
    fn theme_token_values_reset_when_levels_are_replaced() {
        // Given
        set_theme_token_values(
            BTreeMap::from([("shared".into(), "8px".into())]),
            BTreeMap::from([("shared".into(), "0 1px 2px black".into())]),
        );
        // When
        set_theme_token_levels(
            BTreeMap::from([("shared".into(), vec![0, 2])]),
            BTreeMap::from([("shared".into(), vec![0, 3])]),
        );
        // Then
        assert_eq!(get_first_theme_token_value("width", "$shared"), None);
        assert_eq!(get_first_theme_token_value("box-shadow", "$shared"), None);
        assert_eq!(get_responsive_theme_token("$shared"), Some(vec![0, 2]));
        set_theme_token_levels(BTreeMap::new(), BTreeMap::new());
    }

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
    #[serial]
    fn test_color_and_shadow_tokens() {
        set_color_tokens(BTreeSet::from(["primary".to_string()]));
        let mut shadow = BTreeMap::new();
        shadow.insert("card".to_string(), vec![0]);
        set_theme_token_levels(BTreeMap::new(), shadow);

        assert!(is_color_token("primary"));
        assert!(!is_color_token("card"));
        assert!(is_shadow_token("card"));
        assert!(!is_shadow_token("primary"));
        set_color_tokens(BTreeSet::new());
        set_theme_token_levels(BTreeMap::new(), BTreeMap::new());
        assert!(!is_color_token("primary"));
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
        assert_eq!(
            crate::set_custom_shorthands(BTreeMap::from([("alias".into(), vec!["left".into()])])),
            Ok(())
        );
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
