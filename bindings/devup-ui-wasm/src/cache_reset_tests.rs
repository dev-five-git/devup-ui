use super::*;

const STATIC: &str = "import {css} from '@devup-ui/react';export const x=css({color:'red'});";
const DYNAMIC: &str =
    "import {Box} from '@devup-ui/react';export const x=(p)=><Box color={p.color}/>;";

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn forged_static_reset_is_cold_when_companions_match_or_are_absent(#[case] matching: bool) {
    // Given: a real complete snapshot with a static declaration disguised as a reset.
    fresh();
    let cold = output("reset.tsx", STATIC, true);
    let mut value = snapshot();
    let property = &mut value["properties"][""]["255"]["0"][0];
    property["r"] = true.into();
    property["v"] = "blue".into();
    fresh();
    if matching {
        companions(&value);
    }
    // When: the corrupt current snapshot reaches admission.
    assert_eq!(import(value), Ok(()));
    // Then: no poisoned declaration survives and whole output is cold.
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    assert_eq!(output("reset.tsx", STATIC, true), cold);
    fresh();
}

enum ResetDamage {
    Value,
    Selector,
    Level,
    Layer,
    Typography,
    Variable,
    Orphan,
}

#[rstest]
#[case(ResetDamage::Value)]
#[case(ResetDamage::Selector)]
#[case(ResetDamage::Level)]
#[case(ResetDamage::Layer)]
#[case(ResetDamage::Typography)]
#[case(ResetDamage::Variable)]
#[case(ResetDamage::Orphan)]
#[serial]
fn corrupt_owner_reset_restores_nothing(
    #[case] damage: ResetDamage,
    #[values(false, true)] seeded: bool,
) {
    // Given: independently cold output and a damaged genuine owner reset.
    fresh();
    if seeded {
        seed_file_map(vec!["reset.tsx".into()]);
    }
    let cold = output("reset.tsx", DYNAMIC, false);
    let mut value = snapshot();
    let properties = value["properties"]["reset.tsx"]["255"]["0"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("properties"));
    let reset = properties
        .iter_mut()
        .find(|property| property["r"] == true)
        .unwrap_or_else(|| panic!("owner reset"));
    match damage {
        ResetDamage::Value => reset["v"] = "inherit".into(),
        ResetDamage::Selector => reset["s"] = serde_json::json!({"Selector": "&:hover"}),
        ResetDamage::Level => {
            let reset = reset.clone();
            properties.retain(|property| property["r"] != true);
            value["properties"]["reset.tsx"]["255"]["1"] = serde_json::json!([reset]);
        }
        ResetDamage::Layer => reset["l"] = "t".into(),
        ResetDamage::Typography => reset["t"] = true.into(),
        ResetDamage::Variable => reset["p"] = "--theme".into(),
        ResetDamage::Orphan => properties.retain(|property| property["r"] == true),
    }
    fresh();
    if seeded {
        seed_file_map(vec!["reset.tsx".into()]);
    }
    // When: malformed reset state is admitted through the serialized API.
    assert_eq!(import(value), Ok(()));
    // Then: all cache state is discarded, including the reset exemption.
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    assert_eq!(output("reset.tsx", DYNAMIC, false), cold);
    fresh();
}

#[test]
#[serial]
fn invalid_reset_claim_does_not_latch_a_false_collision() {
    // Given: a live exact red claim and corrupt descriptor bytes hidden behind r=true.
    fresh();
    output("live.tsx", STATIC, true);
    let before = snapshot();
    let mut value = before.clone();
    value["properties"][""]["255"]["0"][0]["r"] = true.into();
    value["names"]["OLcolor-vred"]["descriptor"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("descriptor"))
        .push(99.into());
    // When: the invalid snapshot overlaps a genuine live claim.
    assert_eq!(import(value), Ok(()));
    // Then: the live sheet remains unchanged and extraction is not poisoned by a latch.
    assert_eq!(snapshot(), before);
    assert!(
        code_extract_internal(
            "later.tsx",
            STATIC,
            "@devup-ui/react",
            "df".into(),
            true,
            false,
            false,
            HashMap::new()
        )
        .is_ok()
    );
    fresh();
}
