use crate::{
    counter_test_support::{FixtureRequest, with_original},
    extract_style::{ExtractDynamicStyle, ProducerPolicy},
    sparse_sites::{SiteScope, producer_policy, site_at, site_errors},
};
use css::file_map::{get_original_ids, set_original_ids};
use std::{collections::BTreeMap, panic::catch_unwind};

#[test]
#[serial_test::serial]
fn nesting_restores_prior_counter_when_inner_constructor_scope_succeeds() {
    // Given: an existing private numbered scope with prior assignment state.
    let saved = get_original_ids();
    set_original_ids(BTreeMap::new());
    let _prior = SiteScope::enter_counter_numbered(42, "prior", &[]);
    let prior_site = site_at(1, 2, "prior");
    // When: nested fixtures construct owned IR from different raw originals.
    let (outer, inner, restored) = with_original(
        FixtureRequest {
            filename: "a",
            source: "tone",
        },
        || {
            let outer = ExtractDynamicStyle::new("color", 0, "tone", None).at(0);
            let inner = with_original(
                FixtureRequest {
                    filename: "b",
                    source: "other",
                },
                || ExtractDynamicStyle::new("color", 0, "other", None).at_role(1, 3),
            )
            .unwrap_or_else(|error| panic!("inner: {error}"));
            (outer, inner, producer_policy())
        },
    )
    .unwrap_or_else(|error| panic!("outer: {error}"));
    // Then: both nested and pre-existing contexts restore, including assignments.
    assert_eq!(outer.producer_policy(), ProducerPolicy::CounterOriginal(0));
    assert_eq!(inner.producer_policy(), ProducerPolicy::CounterOriginal(1));
    assert_eq!(restored, ProducerPolicy::CounterOriginal(0));
    assert_eq!(producer_policy(), ProducerPolicy::CounterOriginal(42));
    assert_eq!(site_at(1, 2, "prior"), prior_site);
    assert_eq!(site_errors(), vec![]);
    set_original_ids(saved);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn closure_error_restores_context_when_prior_policy_is_current_or_counter(#[case] counter: bool) {
    // Given: either ordinary Current or private Counter scope has prior sites.
    let saved = get_original_ids();
    set_original_ids(BTreeMap::new());
    let _prior = if counter {
        SiteScope::enter_counter_numbered(42, "prior", &[])
    } else {
        SiteScope::enter("prior", "prior", &[])
    };
    let policy = producer_policy();
    let site = site_at(1, 2, "prior");
    // When: the synchronous closure returns its own typed error.
    let result = with_original(
        FixtureRequest {
            filename: "inner",
            source: "tone",
        },
        || {
            assert_eq!(producer_policy(), ProducerPolicy::CounterOriginal(0));
            Err::<(), _>(crate::extract_style::CounterProducerError::WrongPolicy)
        },
    );
    // Then: closure Err is preserved and neither policy nor assignments leak.
    assert_eq!(
        result,
        Ok(Err(crate::extract_style::CounterProducerError::WrongPolicy))
    );
    assert_eq!(producer_policy(), policy);
    assert_eq!(site_at(1, 2, "prior"), site);
    assert_eq!(site_errors(), vec![]);
    set_original_ids(saved);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn unwind_restores_context_when_prior_policy_is_current_or_counter(#[case] counter: bool) {
    // Given: a prior context exists before the feature-only fixture starts.
    let saved = get_original_ids();
    set_original_ids(BTreeMap::new());
    let _prior = if counter {
        SiteScope::enter_counter_numbered(42, "prior", &[])
    } else {
        SiteScope::enter("prior", "prior", &[])
    };
    let policy = producer_policy();
    let site = site_at(1, 2, "prior");
    // When: the closure unwinds through the fixture's existing RAII scope.
    let result = catch_unwind(|| {
        with_original(
            FixtureRequest {
                filename: "inner",
                source: "tone",
            },
            || {
                assert_eq!(producer_policy(), ProducerPolicy::CounterOriginal(0));
                panic!("fixture unwind")
            },
        )
    });
    // Then: the panic propagates, restoring the complete prior TLS context.
    assert!(result.is_err());
    assert_eq!(producer_policy(), policy);
    assert_eq!(site_at(1, 2, "prior"), site);
    assert_eq!(site_errors(), vec![]);
    set_original_ids(saved);
}
