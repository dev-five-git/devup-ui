use super::test_support::*;

#[rstest::rstest]
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
#[serial_test::serial]
fn admission_rejects_record_damage_when_independent_seed_is_unchanged(
    #[case] family: u8,
    #[case] coordinated: bool,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(family);
    let mut value = packet(&mut sheet);
    if family >= 4 {
        let name = value["keyframes"]["a"]
            .as_object()
            .required("frames")
            .keys()
            .next()
            .required("name")
            .clone();
        value["keyframes"]["a"][&name]["from"][0][1] = json!("damaged");
        if coordinated {
            let expansion = &mut witness(&mut value)["proof"]["emission"]["expansion"]["Keyframes"];
            expansion["steps"][0][1][0][1] = json!("damaged");
            expansion["record"]["Keyframes"]["steps"][0][1][0][1] = json!("damaged");
        }
    } else {
        let records = value["properties"]["a"]["255"]["0"]
            .as_array_mut()
            .required("properties");
        let index = records
            .iter()
            .position(|record| record["r"] == false)
            .required("consumer");
        let old = records[index].clone();
        records[index]["v"] = json!("damaged");
        if coordinated {
            let expansion = &mut witness(&mut value)["proof"]["emission"]["expansion"];
            match family {
                0 => expansion["Static"][0]["Property"]["record"]["v"] = json!("damaged"),
                1 | 2 => {
                    expansion["Dynamic"]["consumer"]["Property"]["record"]["v"] = json!("damaged");
                }
                3 => {
                    let members = expansion["Typography"]["members"]
                        .as_array_mut()
                        .required("members");
                    let member = members
                        .iter_mut()
                        .find(|member| member["Property"]["record"] == old)
                        .required("matching member");
                    member["Property"]["record"]["v"] = json!("damaged");
                }
                _ => panic!("family"),
            }
        }
    }
    let before = observe(&sheet);
    // When
    let result = admit(value);
    // Then
    assert!(matches!(
        result,
        Err(EvidenceError::Kernel(super::super::KernelError::Coverage))
    ));
    assert_eq!(observe(&sheet), before);
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[serial_test::serial]
fn independent_map_linkage_rejects_when_dense_registry_or_group_is_damaged(#[case] damage: u8) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(0);
    let mut value = packet(&mut sheet);
    match damage {
        0 => {
            let slots = value["classMap"]["D9-0"].as_object_mut().required("slots");
            let key = slots.keys().next().required("key").clone();
            let slot = slots.remove(&key).required("slot");
            slots.insert("wrong-key".into(), slot);
        }
        1 => {
            let slots = value["classMap"]["D9-0"].as_object_mut().required("slots");
            let key = slots.keys().next().required("key").clone();
            slots.insert("unused".into(), json!(0));
            slots.insert(key, json!(1));
        }
        2 => value["sourceIds"] = json!({"a":1,"unused":0}),
        3 => value["fileMap"] = json!({"a":1,"unused":0}),
        4 => {
            witness(&mut value)["proof"]["allocation"]["allocation"]["address"]["Counter"]["namespace"] =
                json!("wrong");
        }
        _ => panic!("damage"),
    }
    let before = observe(&sheet);
    // When
    let result = admit(value);
    // Then
    assert!(matches!(
        result,
        Err(EvidenceError::Kernel(super::super::KernelError::Authority))
    ));
    assert_eq!(observe(&sheet), before);
}
