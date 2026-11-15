use css::resident_owner::OwnerFault;
use rstest::rstest;
use serial_test::serial;

use super::owner_native_tests::*;

#[rstest]
#[case::takeover(false)]
#[case::open_targeted_clear(true)]
#[serial]
fn checkpoint_pair_owner_resume_preserves_css_and_frozen_eligibility(
    #[case] targeted_clear: bool,
) -> Result<(), OwnerFault> {
    // Given: the real interpreter owns a nonempty, independently frozen engine pair.
    let _fixture = Fixture::new()?;
    let a = create();
    let b = create();
    ok(OWNER.begin(a));
    ok(OWNER.configured());
    let before = crate::checkpoint_pair_tests::frozen_pair_fixture();
    ok(OWNER.finish());
    // When: production capture/parse/restore handles takeover or open targeted clear/resume.
    if targeted_clear {
        ok(OWNER.clear(a));
        assert!(!ok(OWNER.disposed(a)));
    } else {
        run(b);
    }
    assert!(EVENTS.with_borrow(|events| events.iter().any(|event| event == "sheet")));
    run(a);
    crate::set_atom_hoist(Some(2));
    crate::import_file_routes_internal(std::collections::HashMap::new());
    // Then: restore keeps pre-export CSS and the old global plan despite changed routes.
    assert_eq!(crate::configuration::css_internal(None, false), before);
    assert_eq!(
        crate::with_style_sheet(|sheet| sheet.atom_plan.clone()),
        None
    );
    assert!(css::atom_hoist::is_hoisted_bucket("shared.tsx"));
    assert!(EVENTS.with_borrow(|events| events.iter().any(|event| event == "restore")));
    Ok(())
}
