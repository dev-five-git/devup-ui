use super::*;
use serial_test::serial;
use std::collections::{BTreeSet, HashSet};

pub(crate) fn frozen_pair_fixture() -> String {
    reset_build_state_internal();
    set_atom_hoist(Some(2));
    import_file_routes_internal(HashMap::from([(
        "shared.tsx".into(),
        HashSet::from([0, 1]),
    )]));
    for file in ["first.tsx", "second.tsx"] {
        code_extract_internal(
            file,
            "import {globalCss} from '@devup-ui/react'; globalCss({body:{color:'red'}});",
            "@devup-ui/react",
            "df".into(),
            true,
            false,
            false,
            HashMap::new(),
            None,
        )
        .unwrap_or_else(|error| panic!("{error}"));
    }
    assert_eq!(css::atom_hoist::atom_plan(), None);
    assert_eq!(with_style_sheet(|sheet| sheet.atom_plan.clone()), None);
    let before = configuration::css_internal(None, false);
    assert_eq!(before.matches("color:red").count(), 2);
    let failed = code_extract_internal(
        "shared.tsx",
        "import {css} from '@devup-ui/react'; export const x=css({color:window.name});",
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
        None,
    );
    assert!(failed.is_err());
    assert_eq!(
        css::atom_hoist::atom_plan(),
        Some(BTreeSet::from(["shared.tsx".into()]))
    );
    assert_eq!(with_style_sheet(|sheet| sheet.atom_plan.clone()), None);
    assert_eq!(configuration::css_internal(None, false), before);
    before
}

#[test]
#[serial]
fn checkpoint_pair_export_preserves_live_css_when_global_freezes_before_error() {
    let _scope = TestStateGuard::new();
    // Given: nonempty sheet emission and current-global plans differ after real extraction.
    let before = frozen_pair_fixture();
    // When: a read exports the checkpoint.
    let _checkpoint = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    // Then: read admission must not change the live CSS or either plan.
    assert_eq!(configuration::css_internal(None, false), before);
    assert_eq!(with_style_sheet(|sheet| sheet.atom_plan.clone()), None);
    assert_eq!(
        css::atom_hoist::atom_plan(),
        Some(BTreeSet::from(["shared.tsx".into()]))
    );
}

#[test]
#[serial]
fn checkpoint_pair_roundtrip_preserves_pre_export_css_and_frozen_eligibility() {
    let _scope = TestStateGuard::new();
    // Given: expected CSS is captured BEFORE the export, not normalized by it.
    let before = frozen_pair_fixture();
    let checkpoint = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    reset_build_state_internal();
    // When: direct JSON import restores the pair, then routes change.
    import_sheet_json_internal(&checkpoint).unwrap_or_else(|error| panic!("{error}"));
    set_atom_hoist(Some(2));
    import_file_routes_internal(HashMap::new());
    // Then: the old emission bytes and the independent frozen eligibility both survive.
    assert_eq!(configuration::css_internal(None, false), before);
    assert_eq!(with_style_sheet(|sheet| sheet.atom_plan.clone()), None);
    assert!(css::atom_hoist::is_hoisted_bucket("shared.tsx"));
}
