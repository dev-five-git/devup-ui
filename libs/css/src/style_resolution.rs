//! The `StyleX` `styleResolution` policy a build reads `stylex.create()` with.
//! Devup's own APIs ignore it.

use std::sync::atomic::{AtomicU8, Ordering};

/// How `StyleX` decides between a shorthand and the longhands it covers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StyleResolution {
    /// A longhand beats a shorthand whatever the order (the `StyleX` default)
    #[default]
    PropertySpecificity,
    /// The property applied last wins, shorthand or not
    ApplicationOrder,
    /// Shorthands are expanded into longhands at compile time
    LegacyExpandShorthands,
}

/// Every policy, indexed by the number the global stores
const POLICIES: [StyleResolution; 3] = [
    StyleResolution::PropertySpecificity,
    StyleResolution::ApplicationOrder,
    StyleResolution::LegacyExpandShorthands,
];

impl StyleResolution {
    /// The option value naming the policy
    pub const fn name(self) -> &'static str {
        match self {
            Self::PropertySpecificity => "property-specificity",
            Self::ApplicationOrder => "application-order",
            Self::LegacyExpandShorthands => "legacy-expand-shorthands",
        }
    }

    /// The policy an option value names
    pub fn parse(value: &str) -> Result<Self, String> {
        POLICIES
            .into_iter()
            .find(|policy| policy.name() == value)
            .ok_or_else(|| {
                format!(
                    "devup-ui option stylex.styleResolution must be 'property-specificity', 'application-order' or 'legacy-expand-shorthands', got {value:?}"
                )
            })
    }
}

static STYLE_RESOLUTION: AtomicU8 = AtomicU8::new(0);

/// Read `stylex.create()` under `policy` from now on
pub fn set_style_resolution(policy: StyleResolution) {
    STYLE_RESOLUTION.store(policy as u8, Ordering::Relaxed);
}

/// The policy `stylex.create()` is read under
pub fn style_resolution() -> StyleResolution {
    POLICIES[usize::from(STYLE_RESOLUTION.load(Ordering::Relaxed))]
}

/// Install the policy an option value names; nothing names the default
pub fn set_style_resolution_option(value: Option<&str>) -> Result<(), String> {
    let policy = value.map_or_else(|| Ok(StyleResolution::default()), StyleResolution::parse)?;
    set_style_resolution(policy);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serial_test::serial;

    #[rstest]
    #[case(None, StyleResolution::PropertySpecificity)]
    #[case(Some("property-specificity"), StyleResolution::PropertySpecificity)]
    #[case(Some("application-order"), StyleResolution::ApplicationOrder)]
    #[case(
        Some("legacy-expand-shorthands"),
        StyleResolution::LegacyExpandShorthands
    )]
    #[serial]
    fn option_values_name_their_policy(
        #[case] value: Option<&str>,
        #[case] expected: StyleResolution,
    ) {
        let _state = crate::test_state::TestStateGuard::new();
        set_style_resolution(StyleResolution::ApplicationOrder);
        assert_eq!(set_style_resolution_option(value), Ok(()));
        assert_eq!(style_resolution(), expected);
        assert_eq!(StyleResolution::parse(expected.name()), Ok(expected));
    }

    #[rstest]
    #[case("")]
    #[case("Application-Order")]
    #[case("last-wins")]
    #[serial]
    fn unknown_values_are_config_errors_that_keep_the_policy(#[case] value: &str) {
        let _state = crate::test_state::TestStateGuard::new();
        set_style_resolution(StyleResolution::ApplicationOrder);
        assert_eq!(
            set_style_resolution_option(Some(value)),
            Err(format!(
                "devup-ui option stylex.styleResolution must be 'property-specificity', 'application-order' or 'legacy-expand-shorthands', got {value:?}"
            ))
        );
        assert_eq!(style_resolution(), StyleResolution::ApplicationOrder);
    }
}
