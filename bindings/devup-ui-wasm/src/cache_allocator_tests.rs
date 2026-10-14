use super::*;
use std::fmt::Write;

const SOURCE: &str =
    "import {Box} from '@devup-ui/react';export const x=(p)=><Box color='red' p={2} w={p.width}/>;";

enum MapDamage {
    ClassGap,
    FileGap,
    SourceGap,
    SwappedSlots,
    MissingClasses,
    MissingSources,
    WrongKey,
    WrongVariableSource,
}

#[rstest]
#[case(MapDamage::ClassGap)]
#[case(MapDamage::FileGap)]
#[case(MapDamage::SourceGap)]
#[case(MapDamage::SwappedSlots)]
#[case(MapDamage::MissingClasses)]
#[case(MapDamage::MissingSources)]
#[case(MapDamage::WrongKey)]
#[case(MapDamage::WrongVariableSource)]
#[serial]
fn invalid_allocator_snapshot_is_cold_with_matching_or_absent_companions(
    #[case] damage: MapDamage,
    #[values(false, true)] matching: bool,
) {
    // Given: real counter declarations and their complete self-contained allocator snapshot.
    fresh();
    seed_file_map(vec!["fresh.tsx".into(), "other.tsx".into()]);
    let cold = output("fresh.tsx", SOURCE, false);
    let mut value = snapshot();
    match damage {
        MapDamage::ClassGap => {
            let classes = value["classMap"]["D9-0"]
                .as_object_mut()
                .unwrap_or_else(|| panic!("counter namespace"));
            *classes
                .values_mut()
                .find(|id| **id == 0)
                .unwrap_or_else(|| panic!("slot zero")) = 99.into();
        }
        MapDamage::FileGap => value["fileMap"]["fresh.tsx"] = 99.into(),
        MapDamage::SourceGap => value["sourceIds"]["fresh.tsx"] = 99.into(),
        MapDamage::SwappedSlots => {
            for id in value["classMap"]["D9-0"]
                .as_object_mut()
                .unwrap_or_else(|| panic!("counter namespace"))
                .values_mut()
            {
                if *id == 0 {
                    *id = 1.into();
                } else if *id == 1 {
                    *id = 0.into();
                }
            }
        }
        MapDamage::MissingClasses => value["classMap"] = serde_json::json!({}),
        MapDamage::MissingSources => value["sourceIds"] = serde_json::json!({}),
        MapDamage::WrongKey => {
            let classes = value["classMap"]["D9-0"]
                .as_object_mut()
                .unwrap_or_else(|| panic!("counter namespace"));
            let key = classes
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| panic!("key"));
            let slot = classes.remove(&key).unwrap_or_else(|| panic!("slot"));
            classes.insert("wrong-key".into(), slot);
        }
        MapDamage::WrongVariableSource => {
            for property in value["properties"]["fresh.tsx"]["255"]["0"]
                .as_array_mut()
                .unwrap_or_else(|| panic!("properties"))
            {
                for field in ["p", "v"] {
                    if let Some(text) = property[field].as_str() {
                        property[field] = text.replace("---Sa-", "---Sb-").into();
                    }
                }
            }
        }
    }
    fresh();
    seed_file_map(vec!["fresh.tsx".into(), "other.tsx".into()]);
    let before = snapshot();
    if matching {
        companions(&value);
    }
    // When: invalid allocator state reaches authoritative admission without a companion veto.
    assert_eq!(import(value), Ok(()));
    // Then: fresh numbering survives, no cached declarations are adopted, and full output is cold.
    assert_eq!(snapshot(), before);
    assert_eq!(output("fresh.tsx", SOURCE, false), cold);
    fresh();
}

#[rstest]
#[case(1)]
#[case(31)]
#[serial]
fn current_counter_cache_restores_file_and_slot_ad_blocker_splices(#[case] slots: usize) {
    // Given: a complete real cache crossing both original-file and counter-slot label boundaries.
    let configure = || {
        fresh();
        set_prefix(Some("du-FLa-".into()));
        seed_file_map((0..31).map(|file| format!("{file:02}.tsx")).collect());
    };
    configure();
    let mut elements = String::new();
    for value in 0..slots {
        write!(elements, "<Box color='#{value:06x}'/>").unwrap_or_else(|error| panic!("{error}"));
    }
    let source = format!("import {{Box}} from '@devup-ui/react';export const x=<>{elements}</>; ");
    let cold = output("30.tsx", &source, false);
    let value = snapshot();
    assert!(cold.0.contains("du-FLa-a-d-"), "{}", cold.0);
    configure();
    // When: the valid snapshot is restored without any companion map authority.
    assert_eq!(import(value.clone()), Ok(()));
    // Then: admission keeps the cache and the complete generated output exactly.
    assert_eq!(snapshot(), value);
    assert_eq!(output("30.tsx", &source, false), cold);
    fresh();
}
