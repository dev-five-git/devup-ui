use super::*;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn current_typography_and_keyframe_proofs_round_trip_or_go_cold(#[case] corrupt: bool) {
    // Given: real responsive typography and keyframes with deliberately nonlexical step order.
    reset_build_state_internal();
    register_theme_internal(
        serde_json::from_value(serde_json::json!({
            "typography":{"body":[{"fontSize":"14px","fontFamily":"Sans"},null,{"fontSize":"18px"}]}
        }))
        .unwrap_or_else(|error| panic!("{error}")),
    );
    let source = "import {Box,keyframes} from '@devup-ui/react';export const k=keyframes({to:{opacity:1},from:{opacity:0}});export const x=<Box _hover={{typography:'body'}}/>;";
    let output = code_extract_internal(
        "special.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let cold_css = with_style_sheet(|sheet| sheet.create_css(Some("special.tsx"), false));
    let mut value = snapshot();
    assert!(with_style_sheet(|sheet| sheet
        .properties
        .values()
        .flat_map(|orders| orders.values())
        .flat_map(|levels| levels.values())
        .flatten()
        .any(|property| property.typography)));
    if corrupt {
        let claim = value["names"]
            .as_object_mut()
            .unwrap_or_else(|| panic!("claims"))
            .iter_mut()
            .find(|(name, _)| name.starts_with('K'))
            .unwrap_or_else(|| panic!("keyframes proof"))
            .1;
        claim["descriptor"]
            .as_array_mut()
            .unwrap_or_else(|| panic!("descriptor bytes"))
            .push(99.into());
    }
    reset_build_state_internal();
    // When: complete current data is admitted without changing fresh theme configuration.
    assert_eq!(import(value), Ok(()));
    // Then: valid cached output is preserved, but corrupted proof adopts nothing.
    if corrupt {
        assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    } else {
        assert_eq!(
            with_style_sheet(|sheet| sheet.create_css(Some("special.tsx"), false)),
            cold_css
        );
    }
    let fresh = code_extract_internal(
        "special.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(fresh.code(), output.code());
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("special.tsx"), false)),
        cold_css
    );
    register_theme_internal(sheet::theme::Theme::default());
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn validated_companions_do_not_block_a_later_complete_snapshot(#[case] before: bool) {
    // Given: a previously admitted cache whose companions have been validated.
    reset_build_state_internal();
    seed_file_map(vec!["fresh.tsx".into()]);
    compile();
    let initial = snapshot();
    reset_build_state_internal();
    assert_eq!(import(initial.clone()), Ok(()));
    cache_restore::classes(Some(
        serde_json::from_value(initial["classMap"].clone())
            .unwrap_or_else(|error| panic!("{error}")),
    ));
    cache_restore::files(Some(
        serde_json::from_value(initial["fileMap"].clone())
            .unwrap_or_else(|error| panic!("{error}")),
    ));
    seed_file_map(vec!["later.tsx".into()]);
    code_extract_internal(
        "later.tsx",
        "import {Box} from '@devup-ui/react';export const x=<Box color='blue'/>;",
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let later = snapshot();
    let classes = || {
        Some(
            serde_json::from_value(later["classMap"].clone())
                .unwrap_or_else(|error| panic!("{error}")),
        )
    };
    let files = || {
        Some(
            serde_json::from_value(later["fileMap"].clone())
                .unwrap_or_else(|error| panic!("{error}")),
        )
    };
    // When: existing APIs restore the next complete snapshot in either order.
    if before {
        cache_restore::classes(classes());
        cache_restore::files(files());
    }
    assert_eq!(import(later.clone()), Ok(()));
    if !before {
        cache_restore::classes(classes());
        cache_restore::files(files());
    }
    // Then: no old companion observation vetoes or partially restores the new snapshot.
    assert_eq!(snapshot(), later);
    reset_build_state_internal();
}
