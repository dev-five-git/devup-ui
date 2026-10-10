use super::*;

#[path = "cache_typography_descriptor_tests.rs"]
mod descriptors;

fn configure(numbered: bool) {
    fresh();
    register_theme_internal(
        serde_json::from_value(serde_json::json!({
            "typography":{"body":[{"fontSize":"14px","fontFamily":"Sans"},null,{"fontSize":"18px"}]}
        }))
        .unwrap_or_else(|error| panic!("{error}")),
    );
    if numbered {
        seed_file_map(vec!["fresh.tsx".into()]);
    }
}

fn source(responsive: bool) -> &'static str {
    if responsive {
        "import {Box} from '@devup-ui/react';export const x=<Box _hover={[null,null,{typography:'body'}]}/>;"
    } else {
        "import {Box} from '@devup-ui/react';export const x=<Box _hover={{typography:'body'}}/>;"
    }
}

#[rstest]
#[serial]
fn current_typography_restores_exact_state_without_fresh_theme_authority(
    #[values(false, true)] numbered: bool,
    #[values(false, true)] responsive: bool,
    #[values(false, true)] matching: bool,
) {
    // Given: actual preset expansion, including the original-owner counter path.
    configure(numbered);
    let cold = output("fresh.tsx", source(responsive), false);
    let value = snapshot();
    assert!(cold.2.contains("font-size:18px"), "{}", cold.2);
    if numbered {
        assert!(cold.0.contains("a-a"), "{}", cold.0);
        assert_eq!(
            value["classMap"]["D9-0"]
                .as_object()
                .unwrap_or_else(|| panic!("counter class map"))
                .len(),
            1
        );
    }
    fresh();
    let configured_theme = with_style_sheet(|sheet| sheet.theme.typography.len());
    if matching {
        companions(&value);
    }
    // When: the cache alone proves its declarations with no current preset available.
    assert_eq!(import(value.clone()), Ok(()));
    // Then: this is admission, not a silent cold miss, and fresh theme stays fresh.
    assert_eq!(snapshot(), value);
    assert_eq!(
        with_style_sheet(|sheet| sheet.theme.typography.len()),
        configured_theme
    );
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("fresh.tsx"), false)),
        cold.2
    );
    register_theme_internal(
        serde_json::from_value(serde_json::json!({
            "typography":{"body":[{"fontSize":"14px","fontFamily":"Sans"},null,{"fontSize":"18px"}]}
        }))
        .unwrap_or_else(|error| panic!("{error}")),
    );
    assert_eq!(output("fresh.tsx", source(responsive), false), cold);
    fresh();
}

#[rstest]
#[serial]
fn corrupt_typography_declarations_are_cold_not_sticky(
    #[values(false, true)] numbered: bool,
    #[values(false, true)] responsive: bool,
) {
    // Given: a real cache with one expanded declaration changed but its identity retained.
    configure(numbered);
    let cold = output("fresh.tsx", source(responsive), false);
    let mut value = snapshot();
    let mut changed = false;
    for orders in value["properties"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("properties object"))
        .values_mut()
    {
        for levels in orders
            .as_object_mut()
            .unwrap_or_else(|| panic!("orders object"))
            .values_mut()
        {
            for properties in levels
                .as_object_mut()
                .unwrap_or_else(|| panic!("levels object"))
                .values_mut()
            {
                for property in properties
                    .as_array_mut()
                    .unwrap_or_else(|| panic!("declaration array"))
                {
                    if property["t"] == true && property["p"] == "font-size" {
                        property["v"] = "99px".into();
                        changed = true;
                    }
                }
            }
        }
    }
    assert!(changed);
    configure(numbered);
    let before = snapshot();
    companions(&value);
    // When: matching companions cannot excuse the corrupt declaration proof.
    assert_eq!(import(value), Ok(()));
    // Then: state and complete JS/global/local CSS are independently cold; no error latched.
    assert_eq!(snapshot(), before);
    assert_eq!(output("fresh.tsx", source(responsive), false), cold);
    assert_eq!(cache_names::check(), Ok(()));
    fresh();
}

#[rstest]
#[case("g0")]
#[case("0g")]
#[case("0")]
#[serial]
fn malformed_counter_identity_is_cold_with_matching_companions(#[case] key: &str) {
    // Given: a real current counter typography cache with a non-hex or truncated slot key.
    configure(true);
    let cold = output("fresh.tsx", source(false), false);
    let mut value = snapshot();
    let classes = value["classMap"]["D9-0"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("counter class map"));
    let previous = classes
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| panic!("counter slot key"));
    let slot = classes
        .remove(&previous)
        .unwrap_or_else(|| panic!("counter slot value"));
    classes.insert(key.into(), slot);
    configure(true);
    let before = snapshot();
    companions(&value);
    // When: the malformed identity reaches the counter proof itself.
    assert_eq!(import(value), Ok(()));
    // Then: no partial adoption or false collision, with full cold output preserved.
    assert_eq!(snapshot(), before);
    assert_eq!(output("fresh.tsx", source(false), false), cold);
    fresh();
}
