use std::{collections::BTreeMap, fmt};

use crate::{
    constant::GLOBAL_STYLE_PROPERTY,
    property_names::CSS_PROPERTY_NAMES,
    utils::{to_camel_case, to_kebab_case},
};

/// A rejected target and its logical position in shorthand configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidShorthandTarget {
    /// The configured shorthand name.
    pub alias: String,
    /// The original target spelling, before normalization.
    pub target: String,
    /// The zero-based position in the target list.
    pub index: usize,
}

impl fmt::Display for InvalidShorthandTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "shorthands[{:?}][{}]: shorthand `{}` cannot use `{}` at build time: its targets must be supported CSS properties, built-in aliases or custom properties",
            self.alias, self.index, self.alias, self.target
        )
    }
}

impl std::error::Error for InvalidShorthandTarget {}

fn is_custom_property(property: &str) -> bool {
    property.strip_prefix("--").is_some_and(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-') || c >= '\u{80}')
    })
}

fn normalize_property(property: &str) -> Option<String> {
    if is_custom_property(property) || CSS_PROPERTY_NAMES.contains(property) {
        return Some(property.to_string());
    }
    let kebab = to_kebab_case(property);
    let canonical = if ["Webkit", "Moz", "ms", "O"].iter().any(|prefix| {
        property.strip_prefix(prefix).is_some_and(|suffix| {
            suffix
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_uppercase)
        })
    }) {
        format!("-{kebab}")
    } else {
        kebab.into_owned()
    };
    if !CSS_PROPERTY_NAMES.contains(canonical.as_str()) {
        return None;
    }
    let camel = to_camel_case(&canonical);
    let expected = camel
        .strip_prefix("Ms")
        .map_or_else(|| camel.to_string(), |suffix| format!("ms{suffix}"));
    (property == expected).then_some(canonical)
}

/// Validate every target before producing a normalized registry.
///
/// # Errors
/// Returns the original alias, target and index of the first unsupported target.
pub fn normalize_shorthands(
    shorthands: BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, Vec<String>>, InvalidShorthandTarget> {
    shorthands
        .into_iter()
        .map(|(alias, targets)| {
            let mut normalized = Vec::new();
            for (index, target) in targets.into_iter().enumerate() {
                if let Some(properties) = GLOBAL_STYLE_PROPERTY.get(target.as_str()) {
                    normalized.extend(properties.iter().map(|property| (*property).to_string()));
                } else {
                    normalized.push(normalize_property(&target).ok_or_else(|| {
                        InvalidShorthandTarget {
                            alias: alias.clone(),
                            target,
                            index,
                        }
                    })?);
                }
            }
            Ok((alias, normalized))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serial_test::serial;

    #[rstest]
    #[case("marginRight", Some("margin-right"))]
    #[case("margin-right", Some("margin-right"))]
    #[case("left", Some("left"))]
    #[case("WebkitMaskImage", Some("-webkit-mask-image"))]
    #[case("-webkit-mask-image", Some("-webkit-mask-image"))]
    #[case("MozAppearance", Some("-moz-appearance"))]
    #[case("msUserSelect", Some("-ms-user-select"))]
    #[case("-ms-user-select", Some("-ms-user-select"))]
    #[case("OTransform", None)]
    #[case("--My-color", Some("--My-color"))]
    #[case("--1", Some("--1"))]
    #[case("--색", Some("--색"))]
    #[case("--", None)]
    #[case("--bad name", None)]
    #[case("width ", None)]
    #[case("Width", None)]
    #[case("widht", None)]
    #[case("scrollMargnLeft", None)]
    #[case("webkitMaskImage", None)]
    #[case("MsUserSelect", None)]
    #[case("styleOrder", None)]
    #[case("_hover", None)]
    #[case("positioning", None)]
    #[case("", None)]
    fn validates_spelling(#[case] property: &str, #[case] expected: Option<&str>) {
        assert_eq!(normalize_property(property).as_deref(), expected);
    }

    #[test]
    fn rejects_unprefixed_dead_properties() {
        for property in [
            "box-align",
            "box-pack",
            "box-flex",
            "box-flex-group",
            "box-orient",
            "box-ordinal-group",
            "box-direction",
            "box-lines",
            "flex-order",
            "flex-positive",
            "flex-negative",
            "flex-preferred-size",
            "scroll-snap-coordinate",
            "scroll-snap-destination",
            "scroll-snap-points-x",
            "scroll-snap-points-y",
            "scroll-snap-type-x",
            "scroll-snap-type-y",
        ] {
            assert_eq!(normalize_property(property), None);
            assert_eq!(normalize_property(&to_camel_case(property)), None);
        }
    }

    #[test]
    fn preserves_builtin_order_and_empty_lists() -> Result<(), InvalidShorthandTarget> {
        let normalized = normalize_shorthands(BTreeMap::from([
            (
                "edges".into(),
                vec![
                    "left".into(),
                    "py".into(),
                    "margin-right".into(),
                    "--Gap".into(),
                ],
            ),
            ("empty".into(), vec![]),
        ]))?;
        assert_eq!(
            normalized["edges"],
            [
                "left",
                "padding-top",
                "padding-bottom",
                "margin-right",
                "--Gap"
            ]
        );
        assert_eq!(normalized["empty"], Vec::<String>::new());
        Ok(())
    }

    #[test]
    #[serial]
    fn invalid_map_preserves_previous_registry() -> Result<(), InvalidShorthandTarget> {
        crate::set_custom_shorthands(BTreeMap::from([("old".into(), vec!["left".into()])]))?;
        let error = match crate::set_custom_shorthands(BTreeMap::from([
            ("aValid".into(), vec!["right".into()]),
            ("bad".into(), vec!["width".into(), "widht".into()]),
        ])) {
            Err(error) => error,
            Ok(()) => panic!("expected invalid shorthand registration to fail"),
        };
        assert_eq!(
            error,
            InvalidShorthandTarget {
                alias: "bad".into(),
                target: "widht".into(),
                index: 1
            }
        );
        assert_eq!(
            error.to_string(),
            "shorthands[\"bad\"][1]: shorthand `bad` cannot use `widht` at build time: its targets must be supported CSS properties, built-in aliases or custom properties"
        );
        assert_eq!(crate::get_custom_shorthand_names(), ["old"]);
        assert_eq!(
            crate::disassemble_property("old").collect::<Vec<_>>(),
            ["left"]
        );
        crate::set_custom_shorthands(BTreeMap::new())?;
        Ok(())
    }

    #[test]
    #[serial]
    fn registration_recovers_a_poisoned_registry() -> Result<(), InvalidShorthandTarget> {
        match std::panic::catch_unwind(|| -> Result<(), Box<dyn std::error::Error>> {
            let _registry = crate::CUSTOM_SHORTHANDS.write()?;
            panic!("poison the shorthand registry");
        }) {
            Err(_) => {}
            Ok(_) => panic!("expected the registry poisoning closure to panic"),
        }
        crate::set_custom_shorthands(BTreeMap::from([("recovered".into(), vec!["right".into()])]))?;
        assert_eq!(
            crate::disassemble_property("recovered").collect::<Vec<_>>(),
            ["right"]
        );
        crate::set_custom_shorthands(BTreeMap::new())?;
        Ok(())
    }
}
