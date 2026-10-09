use crate::{
    counter_test_support::{FixtureRequest, with_original},
    extract_style::{
        ExtractDynamicStyle, ExtractKeyframes, ProducerPolicy,
        extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
    },
};
use css::file_map::{get_original_ids, set_original_ids};
use std::collections::BTreeMap;

#[test]
#[serial_test::serial]
fn registration_preserves_exact_filename_when_restored_id_is_not_length() {
    // Given: a restored ID is unrelated to registry length.
    let saved = get_original_ids();
    set_original_ids(BTreeMap::from([("seed.tsx".into(), 42)]));
    // When: constructors register repeated and case-distinct raw filenames.
    let policies = ["seed.tsx", "seed.tsx", "Seed.tsx", "./seed.tsx"].map(|filename| {
        with_original(
            FixtureRequest {
                filename,
                source: "tone",
            },
            || ExtractStaticStyle::new("color", "red", 0, None).producer_policy(),
        )
        .unwrap_or_else(|error| panic!("fixture: {error}"))
    });
    // Then: exact reuse preserves 42; new raw spellings append at length.
    assert_eq!(
        policies,
        [
            ProducerPolicy::CounterOriginal(42),
            ProducerPolicy::CounterOriginal(42),
            ProducerPolicy::CounterOriginal(1),
            ProducerPolicy::CounterOriginal(2),
        ]
    );
    assert_eq!(
        get_original_ids(),
        BTreeMap::from([
            ("seed.tsx".into(), 42),
            ("Seed.tsx".into(), 1),
            ("./seed.tsx".into(), 2),
        ])
    );
    set_original_ids(saved);
}

#[test]
#[serial_test::serial]
fn numeric_sites_match_entry_normalization_when_received_source_has_bom_and_crlf() {
    // Given: both sources parse as the same normalized LF source.
    let saved = get_original_ids();
    set_original_ids(BTreeMap::new());
    // When: real constructors attach the normalized AST offset and syntax role.
    let styles = ["\u{feff}a\r\nbcd", "a\nbcd"].map(|source| {
        with_original(
            FixtureRequest {
                filename: "raw.tsx",
                source,
            },
            || ExtractDynamicStyle::new("color", 0, "tone", None).at_role(2, 3),
        )
        .unwrap_or_else(|error| panic!("fixture: {error}"))
    });
    // Then: normalization occurs once, retaining numbered position and role.
    let expected = css::Site {
        file: css::sparse_site::SourceFile::D9(0),
        at: 2,
        role: 3,
    };
    assert_eq!(styles[0].site(), Some(&expected));
    assert_eq!(styles[1].site(), Some(&expected));
    assert_eq!(
        styles[0].producer_policy(),
        ProducerPolicy::CounterOriginal(0)
    );
    set_original_ids(saved);
}

#[test]
#[serial_test::serial]
fn constructors_retain_policy_before_union_when_fixtures_are_deferred() {
    // Given: real constructors, including basic/static, no-site and empty parents.
    let saved = get_original_ids();
    set_original_ids(BTreeMap::new());
    let values = with_original(
        FixtureRequest {
            filename: "a",
            source: "tone",
        },
        || {
            let static_style = ExtractStaticStyle::new("color", "red", 0, None);
            let basic = ExtractStaticStyle::new_basic("color", "red", 0, None);
            let dynamic = ExtractDynamicStyle::new("color", 0, "tone", None);
            let site = dynamic.clone().at_role(0, 1);
            let empty = ExtractKeyframes::default();
            let mut populated = ExtractKeyframes::default();
            populated.keyframes.insert("to".into(), vec![basic.clone()]);
            assert_eq!(
                static_style.producer_policy(),
                ProducerPolicy::CounterOriginal(0)
            );
            assert_eq!(basic.producer_policy(), ProducerPolicy::CounterOriginal(0));
            assert_eq!(
                dynamic.producer_policy(),
                ProducerPolicy::CounterOriginal(0)
            );
            assert_eq!(site.producer_policy(), ProducerPolicy::CounterOriginal(0));
            assert_eq!(empty.producer_policy(), ProducerPolicy::CounterOriginal(0));
            assert_eq!(
                populated.producer_policy(),
                ProducerPolicy::CounterOriginal(0)
            );
            vec![
                ExtractStyleValue::Static(static_style),
                ExtractStyleValue::Static(basic),
                ExtractStyleValue::Dynamic(dynamic),
                ExtractStyleValue::Dynamic(site),
                ExtractStyleValue::Keyframes(empty),
                ExtractStyleValue::Keyframes(populated),
            ]
        },
    )
    .unwrap_or_else(|error| panic!("fixture: {error}"));
    // When: the owned unions are cloned after the fixture TLS has ended.
    let deferred = values.clone();
    // Then: the retained policy and numbered site survive; Current is restored.
    assert_eq!(values, deferred);
    for value in deferred {
        match value {
            ExtractStyleValue::Static(style) => {
                assert_eq!(style.producer_policy(), ProducerPolicy::CounterOriginal(0));
            }
            ExtractStyleValue::Dynamic(style) => {
                assert_eq!(style.producer_policy(), ProducerPolicy::CounterOriginal(0));
            }
            ExtractStyleValue::Keyframes(frames) => {
                assert_eq!(frames.producer_policy(), ProducerPolicy::CounterOriginal(0));
            }
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_) => panic!("unexpected constructor union"),
        }
    }
    assert_eq!(
        ExtractKeyframes::default().producer_policy(),
        ProducerPolicy::Current
    );
    set_original_ids(saved);
}
