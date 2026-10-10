use super::*;

#[rstest]
#[case(0)]
#[case(30)]
#[serial]
fn dynamic_counter_owner_round_trip_keeps_the_validated_label(#[case] owner: usize) {
    // Given: real dynamic declarations at the plain and ad-blocker owner-label boundaries.
    let configure = || {
        fresh();
        set_prefix(Some("du-FLa-".into()));
        seed_file_map((0..=owner).map(|id| format!("{id:02}.tsx")).collect());
    };
    configure();
    let file = format!("{owner:02}.tsx");
    let source = "import {Box} from '@devup-ui/react';export const x=(p)=><Box color={p.color}/>;";
    let cold = output(&file, source, false);
    let value = snapshot();
    assert!(cold.2.contains("color:var(---du-FLa-S"), "{}", cold.2);
    configure();
    // When: current admission reuses the already validated owner label for variable proof.
    assert_eq!(import(value.clone()), Ok(()));
    // Then: the complete allocator/reset/declaration snapshot and full output round trip.
    assert_eq!(snapshot(), value);
    assert_eq!(output(&file, source, false), cold);
    fresh();
}

#[test]
#[serial]
fn hoisted_declaration_without_bucket_proof_is_cold() {
    // Given: a genuine current atom whose stored placement falsely claims hoisting.
    fresh();
    let source = "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;";
    let cold = output("fresh.tsx", source, false);
    let mut value = snapshot();
    value["properties"]["fresh.tsx"]["255"]["0"][0]["h"] = true.into();
    fresh();
    let before = snapshot();
    companions(&value);
    // When: admission checks actual placement against the frozen plan.
    assert_eq!(import(value), Ok(()));
    // Then: the entire snapshot remains cold and extraction remains usable.
    assert_eq!(snapshot(), before);
    assert_eq!(output("fresh.tsx", source, false), cold);
    fresh();
}

#[test]
#[serial]
fn invalid_manual_generated_claim_does_not_replace_live_sheet() {
    // Given: authored manual controls plus a generated-looking name lacking its claim.
    fresh();
    let mut live = StyleSheet::default();
    live.add_property(
        "manual-card",
        "color",
        0,
        "red",
        None,
        None,
        Some("fresh.tsx"),
    );
    assert_eq!(import_sheet_internal(live), Ok(()));
    let before = snapshot();
    let css = with_style_sheet(|sheet| sheet.create_css(Some("fresh.tsx"), false));
    assert!(
        css.contains(&format!(".manual-card{{color:{}}}", "red")),
        "{css}"
    );
    let mut incoming = StyleSheet::default();
    incoming.add_property("OLcolor-vblue", "color", 0, "blue", None, Some(0), None);
    // When: in-memory manual import still enforces the generated-name contract.
    assert_eq!(import_sheet_internal(incoming), Ok(()));
    // Then: authored controls stay intact and the rejected claim is not sticky.
    assert_eq!(snapshot(), before);
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("fresh.tsx"), false)),
        css
    );
    assert_eq!(cache_names::check(), Ok(()));
    fresh();
}

#[test]
#[serial]
fn wrong_keyframe_descriptor_variant_is_cold() {
    // Given: actual keyframe output, with an atom variant substituted in the frame proof.
    fresh();
    let source = "import {keyframes} from '@devup-ui/react';export const x=keyframes({from:{opacity:0},to:{opacity:1}});";
    let cold = output("fresh.tsx", source, false);
    let mut value = snapshot();
    let claim = value["names"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("names object"))
        .iter_mut()
        .find(|(name, _)| name.starts_with('K'))
        .unwrap_or_else(|| panic!("keyframe claim"))
        .1;
    claim["descriptor"][1] = 1.into();
    fresh();
    let before = snapshot();
    // When: exact frame decoding rejects the wrong semantic variant.
    assert_eq!(import(value), Ok(()));
    // Then: no cached state is installed and complete binding output is cold.
    assert_eq!(snapshot(), before);
    assert_eq!(output("fresh.tsx", source, false), cold);
    fresh();
}
