use crate::cache6_test_support::*;

fn empty_export(plan: BTreeSet<String>) -> Vec<u8> {
    fresh();
    update(
        &Default::default(),
        OutputRoute {
            raw_source: "empty",
            single_css: true,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    css::atom_hoist::restore_atom_plan(Some(plan.clone()));
    with_style_sheet_mut(|sheet| sheet.atom_plan = Some(plan));
    encoded()
}

fn cold_before(plan: Option<BTreeSet<String>>) {
    fresh();
    css::atom_hoist::restore_atom_plan(plan.clone());
    with_style_sheet_mut(|sheet| {
        sheet.atom_plan = plan;
        sheet.add_css("prior", concat!("body", "{", "color:blue", "}"));
        sheet.add_import("prior", "prior.css");
    });
    css::class_map::with_class_map_mut(|map| {
        map.insert("prior".into(), HashMap::from([("reserved".into(), 0)]));
    });
    seed_file_map(vec!["prior-seed".into()]);
}

#[test]
#[serial]
fn replacement_rejects_q_when_validation_must_observe_installed_p_before_retirement() {
    // Given: independently kernel-owned empty exports, no candidate-derived settings.
    let _guard = Guard::new();
    let p = empty_export(BTreeSet::new());
    let q = empty_export(BTreeSet::from(["q".into()]));
    cold_before(None);
    let expected_cold = authority();
    assert_eq!(cache6_restore::import(&p), Ok(()));
    assert_eq!(css::atom_hoist::atom_plan(), Some(BTreeSet::new()));
    assert!(matches!(
        snapshot6::parse(&q).and_then(snapshot6::validate_snapshot),
        Err(EvidenceError::Configuration)
    ));
    // When
    assert_eq!(cache6_restore::import(&q), Ok(()));
    // Then: rejection retires P, never adopts Q into the unconstrained cold target.
    assert_eq!(authority(), expected_cold);
    cache6_restore::RESTORE.with_borrow(|state| {
        assert_eq!(state, &cache6_session::Restore::default());
    });
}

#[test]
#[serial]
fn replacement_stays_cold_when_consuming_install_refuses_after_prior_retirement() {
    // Given: independent cold replay includes both plans, full typed state and maps.
    let _guard = Guard::new();
    let p = empty_export(BTreeSet::new());
    let q = empty_export(BTreeSet::from(["q".into()]));
    let seeds = vec![
        vec!["b".into(), "b".into()],
        vec!["a".into(), "b".into()],
        vec!["c".into(), "a".into(), "c".into()],
    ];
    cold_before(Some(BTreeSet::new()));
    for batch in &seeds {
        seed_file_map(batch.clone());
    }
    let expected_cold = authority();
    cold_before(Some(BTreeSet::new()));
    assert_eq!(cache6_restore::import(&p), Ok(()));
    for batch in &seeds {
        seed_file_map(batch.clone());
    }
    // Ordinary global plan transition: target remains unlatched, no authority install.
    css::atom_hoist::restore_atom_plan(None);
    assert_eq!(with_style_sheet(snapshot6::check_install_target), Ok(()));
    assert!(
        snapshot6::parse(&q)
            .and_then(snapshot6::validate_snapshot)
            .is_ok()
    );
    // When: validation permits Q under None; retiring P restores independent Some(empty).
    assert_eq!(cache6_restore::import(&q), Ok(()));
    // Then: consuming recheck refuses Q, preserving the complete replayed cold authority.
    assert_eq!(authority(), expected_cold);
    cache6_restore::RESTORE.with_borrow(|state| {
        assert_eq!(state, &cache6_session::Restore::default());
    });
}
