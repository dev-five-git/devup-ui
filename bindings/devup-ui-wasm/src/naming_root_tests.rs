use super::*;
use rstest::rstest;
use serial_test::serial;

const SOURCE: &str = "import {Box} from '@devup-ui/react'; export const x=<Box flexDir={['column','row']} color={tone}/>;";

fn compile(file: &str) -> Output {
    code_extract_internal(
        file,
        SOURCE,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

fn snapshot() -> StyleSheet {
    serde_json::from_str(&export_sheet_internal().unwrap_or_else(|error| panic!("{error}")))
        .unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[serial]
fn configured_prefix_is_not_misread_as_a_generated_scope_in_shared_css() {
    // Given
    reset_build_state_internal();
    set_prefix(Some("du-FLa-".into()));
    code_extract_internal(
        "x.tsx",
        SOURCE,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let incoming = snapshot();
    // When
    let imported = import_sheet_internal(incoming);
    // Then
    assert_eq!(imported, Ok(()));
    reset_build_state_internal();
}

#[rstest]
#[case("du-FLa-")]
#[case("FHprefix-")]
#[serial]
fn configured_prefix_does_not_require_a_fallback_claim_for_a_d9_scope(#[case] prefix: &str) {
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    seed_file_map(vec!["x.tsx".into()]);
    let styles = std::iter::once(ExtractStyleValue::Static(
        extractor::extract_style::extract_static_style::ExtractStaticStyle::new(
            "color", "red", 0, None,
        )
        .with_naming(css::Naming::Risky),
    ))
    .collect();
    with_style_sheet_mut(|sheet| sheet.update_styles(&styles, "x.tsx", false))
        .unwrap_or_else(|error| panic!("{error}"));
    let incoming = snapshot();

    let restored = import_sheet_internal(incoming);

    assert_eq!(restored, Ok(()));
    reset_build_state_internal();
}

#[rstest]
#[case("C:/repo/a", "C:\\repo\\a\\src\\x.tsx", "src/x.tsx")]
#[case("/repo/a", "/repo/a/src/../x.tsx", "x.tsx")]
#[case("/repo/a", "/repo/shared/x.tsx", "../shared/x.tsx")]
#[case("C:/repo/a", "D:\\shared\\x.tsx", "d:/shared/x.tsx")]
#[case("/repo/a", "../shared/x.tsx", "../shared/x.tsx")]
#[case("//server/share/a", "//server/other/x.tsx", "//server/other/x.tsx")]
#[serial]
fn naming_keys_are_relative_when_a_compatible_root_exists(
    #[case] root: &str,
    #[case] file: &str,
    #[case] expected: &str,
) {
    // Given
    reset_build_state_internal();
    set_naming_root(Some(root.into()), None);
    // When
    let key = css::naming_root::key(file);
    // Then
    assert_eq!(key, expected);
    reset_build_state_internal();
}

#[test]
#[serial]
fn no_root_and_reset_preserve_the_existing_raw_key_contract() {
    // Given
    set_naming_root(Some("C:/repo".into()), None);
    reset_build_state_internal();
    // When
    let key = css::naming_root::key("C:\\repo\\x.tsx");
    // Then
    assert_eq!(key, "C:\\repo\\x.tsx");
}

#[test]
#[serial]
fn checkout_roots_do_not_change_full_output_when_only_naming_context_differs() {
    // Given: no D9 IDs, identical source and root-relative file/sheet identities.
    let mut outputs = Vec::new();
    // When: two independent builds use the new channel, not rewritten extraction IDs.
    for root in ["/checkout/one", "/different/place/two"] {
        reset_build_state_internal();
        css::debug::set_debug(false);
        set_naming_root(Some(root.into()), None);
        let file = format!("{root}/src/x.tsx");
        let output = compile(&file);
        let sheet = snapshot();
        assert!(sheet.properties.contains_key(&file));
        let claims = sheet
            .names
            .iter()
            .filter(|(name, _)| name.starts_with('F') && !name.contains('-'))
            .collect::<Vec<_>>();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].1.content, "src/x.tsx");
        assert_eq!(claims[0].1.descriptor, b"src/x.tsx");
        outputs.push((output.code(), sheet.create_css(Some(&file), false)));
    }
    // Then: every emitted JS and CSS byte matches; delivery keys remain absolute.
    assert_eq!(outputs[0], outputs[1]);
    reset_build_state_internal();
}

#[test]
#[serial]
fn cwd_relative_ids_keep_checkout_stable_names_when_compiler_context_differs() {
    // Given: Webpack keeps cwd-relative IDs even when compiler.context is elsewhere.
    let mut outputs = Vec::new();
    // When
    for checkout in ["one", "two"] {
        reset_build_state_internal();
        set_naming_root(
            Some(format!("/checkouts/{checkout}")),
            Some("/runner".into()),
        );
        let file = format!("../checkouts/{checkout}/src/x.tsx");
        let output = compile(&file);
        outputs.push((
            output.code(),
            with_style_sheet(|sheet| {
                assert!(sheet.properties.contains_key(&file));
                sheet.create_css(Some(&file), false)
            }),
        ));
    }
    // Then: resolver/delivery IDs stay raw, but every CSS/JS byte is checkout-stable.
    assert_eq!(outputs[0], outputs[1]);
    reset_build_state_internal();
}

#[test]
#[serial]
fn scope_collision_reports_full_relative_keys_and_both_real_locations() {
    // Given: a narrow width forces two different sheet keys to the same prefix.
    reset_build_state_internal();
    set_naming_root(Some("/checkout".into()), None);
    let bits = css::content_hash::FingerprintBits::new(1).unwrap_or_else(|| panic!("valid width"));
    let first_name =
        css::content_name::ContentName::scope("src/first.tsx").name_with_bits("", bits);
    let second = (0..64)
        .map(|index| format!("src/second{index}.tsx"))
        .find(|key| {
            css::content_name::ContentName::scope(key).name_with_bits("", bits) == first_name
        })
        .unwrap_or_else(|| panic!("narrow scope collision fixture"));
    let second_file = format!("/checkout/{second}");
    let source = "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;";
    let styles =
        extractor::extract_without_source_map(&second_file, source, ExtractOption::default())
            .unwrap_or_else(|error| panic!("{error}"))
            .styles;
    let mut sheet = StyleSheet::default();
    let first_styles = extractor::extract_without_source_map(
        "/checkout/src/first.tsx",
        source,
        ExtractOption::default(),
    )
    .unwrap_or_else(|error| panic!("{error}"))
    .styles;
    sheet.names = sheet
        .preflight_styles_with_bits(&first_styles, ("/checkout/src/first.tsx", false), bits)
        .unwrap_or_else(|error| panic!("{error}"));
    // When
    let error = sheet
        .preflight_styles_with_bits(&styles, (&second_file, false), bits)
        .err()
        .unwrap_or_else(|| panic!("scope collision accepted"))
        .to_string();
    // Then
    for value in [
        "src/first.tsx",
        &second,
        "/checkout/src/first.tsx:1:",
        &format!("{second_file}:1:"),
    ] {
        assert!(error.contains(value), "{error}");
    }
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn cache_rejects_missing_or_corrupt_scope_claims_before_any_extraction(#[case] corrupt: bool) {
    // Given
    reset_build_state_internal();
    set_naming_root(Some("/checkout".into()), None);
    let cold = compile("/checkout/src/x.tsx");
    let cold_css = with_style_sheet(|sheet| sheet.create_css(Some("/checkout/src/x.tsx"), false));
    let mut incoming = snapshot();
    let scope = incoming
        .names
        .keys()
        .find(|name| name.starts_with('F'))
        .unwrap_or_else(|| panic!("missing scope"))
        .clone();
    if corrupt {
        incoming
            .names
            .get_mut(&scope)
            .unwrap_or_else(|| panic!("missing scope"))
            .descriptor
            .push(1);
    } else {
        incoming.names.remove(&scope);
    }
    reset_build_state_internal();
    set_naming_root(Some("/checkout".into()), None);
    // When
    assert_eq!(import_sheet_internal(incoming), Ok(()));
    // Then: corrupt scope proof adopts no aliases; the names-only root stays configured.
    assert_eq!(with_style_sheet(|sheet| sheet.names.len()), 0);
    assert_eq!(compile("/checkout/src/x.tsx").code(), cold.code());
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("/checkout/src/x.tsx"), false)),
        cold_css
    );
    reset_build_state_internal();
}
