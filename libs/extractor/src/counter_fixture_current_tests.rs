use crate::{
    ExtractOption,
    counter_test_support::{FixtureRequest, with_original},
    extract,
    extract_style::{
        CounterProducerError, ExtractDynamicStyle, ProducerPolicy,
        extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
    },
};

#[test]
#[serial_test::serial]
fn ordinary_extract_stays_current_when_called_inside_and_outside_counter_fixture() {
    // Given: feature-enabled ordinary extraction with a static and dynamic declaration.
    css::set_prefix(Some("p".into()));
    css::debug::set_debug(false);
    css::atom_hoist::set_atom_hoist(None);
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::class_map::reset_class_map();
    let source =
        "import { Box } from '@devup-ui/react'; export const a = <Box color='red' width={tone} />;";
    let outside = extract("current.tsx", source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("ordinary: {error}"));
    let before = css::class_map::get_class_map();
    // When: the same real compiler entry runs inside a Counter constructor fixture.
    let (inside, restored) = with_original(
        FixtureRequest {
            filename: "fixture",
            source: "tone",
        },
        || {
            let inside = extract("current.tsx", source, ExtractOption::default())
                .unwrap_or_else(|error| panic!("nested ordinary: {error}"));
            let restored = ExtractDynamicStyle::new("color", 0, "tone", None).at(0);
            (inside, restored)
        },
    )
    .unwrap_or_else(|error| panic!("fixture: {error}"));
    let after = extract("current.tsx", source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("after ordinary: {error}"));
    // Then: byte output and style identities do not change or reserve Counter names.
    assert_eq!(inside.code, outside.code);
    assert_eq!(inside.map, outside.map);
    assert_eq!(inside.css_file, outside.css_file);
    assert_eq!(inside.styles, outside.styles);
    assert_eq!(after.code, outside.code);
    assert_eq!(after.styles, outside.styles);
    assert_eq!(css::class_map::get_class_map(), before);
    assert_eq!(
        restored.producer_policy(),
        ProducerPolicy::CounterOriginal(0)
    );
    assert_eq!(
        restored.site().map(|site| (&site.file, site.at, site.role)),
        Some((&css::sparse_site::SourceFile::D9(0), 0, 0))
    );
    assert_eq!(
        ExtractStaticStyle::new("color", "red", 0, None).producer_policy(),
        ProducerPolicy::Current
    );
    assert_eq!(inside.styles.len(), 2);
    for value in inside.styles {
        match value {
            ExtractStyleValue::Static(style) => {
                assert_eq!(style.producer_policy(), ProducerPolicy::Current);
                assert_eq!(
                    style.counter_produce(None),
                    Err(CounterProducerError::WrongPolicy)
                );
            }
            ExtractStyleValue::Dynamic(style) => {
                assert_eq!(style.producer_policy(), ProducerPolicy::Current);
                assert_eq!(
                    style.counter_produce(None),
                    Err(CounterProducerError::WrongPolicy)
                );
            }
            ExtractStyleValue::Keyframes(_)
            | ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_) => panic!("unexpected ordinary style"),
        }
    }
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::set_prefix(None);
}
