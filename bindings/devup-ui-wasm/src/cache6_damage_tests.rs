use super::cache6_test_support::*;
use serde_json::{Value, json};

fn damage(value: &mut Value, family: u8) {
    if family >= 4 {
        let name = value["keyframes"]["a"]
            .as_object()
            .unwrap_or_else(|| panic!("frames"))
            .keys()
            .next()
            .unwrap_or_else(|| panic!("name"))
            .clone();
        value["keyframes"]["a"][&name]["from"][0][1] = json!("damaged");
        let expansion = &mut value["evidence"]["counters"]["D9-0"]["0"][0]["proof"]["emission"]["expansion"]
            ["Keyframes"];
        expansion["steps"][0][1][0][1] = json!("damaged");
        expansion["record"]["Keyframes"]["steps"][0][1][0][1] = json!("damaged");
    } else {
        let records = value["properties"]["a"]["255"]["0"]
            .as_array_mut()
            .unwrap_or_else(|| panic!("records"));
        let record = records
            .iter_mut()
            .find(|record| record["r"] == false)
            .unwrap_or_else(|| panic!("consumer"));
        let old = record.clone();
        record["v"] = json!("damaged");
        let expansion =
            &mut value["evidence"]["counters"]["D9-0"]["0"][0]["proof"]["emission"]["expansion"];
        match family {
            0 => expansion["Static"][0]["Property"]["record"]["v"] = json!("damaged"),
            1 | 2 => expansion["Dynamic"]["consumer"]["Property"]["record"]["v"] = json!("damaged"),
            3 => {
                let member = expansion["Typography"]["members"]
                    .as_array_mut()
                    .unwrap_or_else(|| panic!("members"))
                    .iter_mut()
                    .find(|member| member["Property"]["record"] == old)
                    .unwrap_or_else(|| panic!("member"));
                member["Property"]["record"]["v"] = json!("damaged");
            }
            _ => panic!("family"),
        }
    }
}

#[rstest]
#[case(0, false)]
#[case(0, true)]
#[case(1, false)]
#[case(1, true)]
#[case(2, false)]
#[case(2, true)]
#[case(3, false)]
#[case(3, true)]
#[case(4, false)]
#[case(4, true)]
#[case(5, false)]
#[case(5, true)]
#[serial]
fn coordinated_record_damage_is_cold_when_independent_seed_is_unchanged(
    #[case] family: u8,
    #[case] supplied: bool,
) {
    // Given
    let _guard = Guard::new();
    configure_typography();
    let cold = output("a", family);
    let bytes = encoded();
    let expected_maps = maps();
    let mut value: Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("{error:?}"));
    damage(&mut value, family);
    let damaged = serde_json::to_vec(&value).unwrap_or_else(|error| panic!("{error:?}"));
    fresh();
    configure_typography();
    if supplied {
        companions(&expected_maps, false);
    }
    let before = authority();
    // When
    assert_eq!(cache6_restore::import(&damaged), Ok(()));
    // Then
    assert_eq!(authority(), before);
    assert_eq!(cache_names::check(), Ok(()));
    assert_eq!(output("a", family), cold);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[case(6)]
#[serial]
fn incompatible_raw_inputs_are_cold_when_version_or_shape_is_wrong(#[case] kind: u8) {
    // Given
    let _guard = Guard::new();
    let cold = output("a", 0);
    let mut value: Value =
        serde_json::from_slice(&encoded()).unwrap_or_else(|error| panic!("{error:?}"));
    let bytes = match kind {
        0 => {
            fresh();
            exact_test_support::compile(
                "current.tsx",
                "import {css} from '@devup-ui/react';export const x=css({color:'blue'});",
            )
            .unwrap_or_else(|error| panic!("{error}"));
            export_sheet_internal()
                .unwrap_or_else(|error| panic!("{error}"))
                .into_bytes()
        }
        1 => {
            value["atomNamingVersion"] = json!(5);
            serde_json::to_vec(&value).unwrap_or_else(|error| panic!("{error}"))
        }
        2 => {
            value["atomNamingVersion"] = json!(7);
            serde_json::to_vec(&value).unwrap_or_else(|error| panic!("{error}"))
        }
        3 => {
            value
                .as_object_mut()
                .unwrap_or_else(|| panic!("object"))
                .remove("atomNamingVersion");
            serde_json::to_vec(&value).unwrap_or_else(|error| panic!("{error}"))
        }
        4 => b"{truncated".to_vec(),
        5 => b"null".to_vec(),
        6 => {
            let raw = String::from_utf8(encoded()).unwrap_or_else(|error| panic!("{error}"));
            raw.replacen(
                "\"atomNamingVersion\":6",
                "\"atomNamingVersion\":6,\"atomNamingVersion\":6",
                1,
            )
            .into_bytes()
        }
        _ => panic!("kind"),
    };
    fresh();
    let before = authority();
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    // Then
    assert_eq!(authority(), before);
    assert_eq!(cache_names::check(), Ok(()));
    assert_eq!(output("a", 0), cold);
}
