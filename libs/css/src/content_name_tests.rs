use super::content_name::{AtomContent, ContentName};
use super::{
    Naming,
    content_hash::fingerprint,
    naming::private_counter,
    style_selector::{AtRule, AtRuleKind, StyleSelector},
};
use serial_test::serial;

fn atom(value: Option<&str>) -> AtomContent<'_> {
    AtomContent {
        property: "color",
        value,
        naming: Naming::Own,
        level: 0,
        order: 255,
        selector: None,
        layer: None,
        dynamic: false,
    }
}

#[test]
fn descriptor_bytes_and_short_names_are_fixed_goldens() {
    // Given
    let input = atom(Some("red"));
    // When
    let content = input.content();
    // Then
    assert_eq!(
        content.descriptor,
        [
            1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 5, b'c', b'o', b'l', b'o', b'r', 1, 0, 0, 0, 0, 0, 0, 0,
            3, b'r', b'e', b'd', 0, 255, 0, 0,
        ]
    );
    assert_eq!(content.name("du-"), "du-OLcolor-vred");
}

#[test]
fn names_choose_the_complete_escaped_minimum_and_lossless_on_ties() {
    // Given
    let descriptor = b"abc".to_vec();
    // When / Then
    for length in [0, 15, 16, 17, 500] {
        let content = ContentName {
            descriptor: descriptor.clone(),
            lossless: "x".repeat(length),
            domain: 'R',
        };
        let name = content.name("prefix-");
        let expected = if length <= 16 {
            format!("prefix-RL{}", "x".repeat(length))
        } else {
            format!("prefix-RH{}", fingerprint(b"abc"))
        };
        assert_eq!(name, expected);
        assert_eq!(name.len(), 9 + length.min(16));
    }
}

#[test]
fn parameterized_width_preserves_prefix_bits_padding_and_lossless_ties() {
    // Given
    use crate::content_hash::{FingerprintBits, fingerprint_with_bits};
    let descriptor = b"abc".to_vec();
    // When / Then
    for (width, expected) in [(1, "b"), (5, "x"), (6, "bj")] {
        let bits = FingerprintBits::new(width).unwrap_or_else(|| panic!("valid width"));
        assert_eq!(fingerprint_with_bits(b"abc", bits), expected);
    }
    for width in 1..=80 {
        let bits = FingerprintBits::new(width).unwrap_or_else(|| panic!("valid width"));
        for length in [bits.digits() - 1, bits.digits(), bits.digits() + 1] {
            let content = ContentName {
                descriptor: descriptor.clone(),
                lossless: "x".repeat(length),
                domain: 'R',
            };
            let name = content.name_with_bits("prefix-", bits);
            assert_eq!(name.len(), 9 + length.min(bits.digits()));
            assert_eq!(name.starts_with("prefix-RL"), length <= bits.digits());
        }
    }
    assert_eq!(FingerprintBits::new(0), None);
    assert_eq!(FingerprintBits::new(81), None);
    assert_eq!(FingerprintBits::PRODUCTION.digits(), 16);
    assert_eq!(
        fingerprint_with_bits(b"abc", FingerprintBits::PRODUCTION),
        fingerprint(b"abc")
    );
}

#[test]
fn selector_structure_and_layer_are_not_flattened_or_cleanup_owned() {
    // Given
    let at = |file| StyleSelector::At {
        kind: AtRuleKind::Media,
        query: "print".into(),
        selector: None,
        outer: vec![AtRule {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".into(),
        }],
        file,
    };
    let first = at(Some("a.tsx".into()));
    let second = at(Some("b.tsx".into()));
    let mut input = atom(Some("red"));
    input.selector = Some(&first);
    // When
    let a = input.content();
    input.selector = Some(&second);
    let b = input.content();
    input.layer = Some("print");
    let layer = input.content();
    // Then
    assert_eq!(a, b);
    assert_ne!(a.descriptor, layer.descriptor);
    assert_ne!(a.name(""), layer.name(""));
}

#[test]
fn semantic_fields_and_variants_remain_separate_even_with_equal_values() {
    // Given
    let mut input = atom(None);
    let missing = input.content();
    // When
    input.value = Some("");
    let empty = input.content();
    input.naming = Naming::Risky;
    let risky = input.content();
    input.dynamic = true;
    let dynamic = input.content();
    // Then
    let names = [
        missing.name(""),
        empty.name(""),
        risky.name(""),
        dynamic.name(""),
    ];
    assert_eq!(
        names
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
}

#[test]
fn keyframes_keep_step_and_declaration_order_in_the_descriptor() {
    // Given
    let mut steps = vec![(
        "from".into(),
        vec![
            ("opacity".into(), "0".into()),
            ("color".into(), "red".into()),
        ],
    )];
    let first = ContentName::keyframes(&steps);
    // When
    steps[0].1.reverse();
    let reversed = ContentName::keyframes(&steps);
    // Then
    assert_ne!(first.descriptor, reversed.descriptor);
    assert_ne!(first.name(""), reversed.name(""));
    assert_eq!(
        ContentName::keyframes(&[]).descriptor,
        [1, 3, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(ContentName::keyframes(&[]).name(""), "KL");
}

#[test]
#[serial]
fn only_original_d9_private_own_atoms_take_counter_slots_even_in_atom_mode() {
    // Given
    crate::file_map::reset_file_map();
    crate::file_map::reset_canonical_map();
    crate::class_map::reset_class_map();
    crate::atom_hoist::restore_atom_plan(None);
    crate::atom_hoist::set_atom_hoist(Some(2));
    crate::file_routes::set_file_routes(std::collections::HashMap::from([(
        "shared.tsx".into(),
        std::collections::HashSet::from([0, 1]),
    )]));
    crate::file_map::seed_file_numbers(&[
        "a.tsx".into(),
        "private.tsx".into(),
        "shared.tsx".into(),
    ]);
    // When
    let private = crate::sheet_to_classname_content(&atom(Some("red")), Some("private.tsx"));
    let shared = crate::sheet_to_classname_content(&atom(Some("red")), Some("shared.tsx"));
    let unnumbered = crate::sheet_to_classname_content(&atom(Some("red")), Some("unknown.tsx"));
    // Then
    assert_eq!(private, "b-a");
    assert_eq!(shared, "OLcolor-vred");
    assert_eq!(unnumbered, "OLcolor-vred");
    assert_eq!(
        private_counter(Some("private.tsx"), Naming::Risky, 255),
        None
    );
    assert_eq!(private_counter(Some("private.tsx"), Naming::Own, 0), None);
    assert_eq!(private_counter(None, Naming::Own, 255), None);
    crate::atom_hoist::set_atom_hoist(None);
    crate::atom_hoist::restore_atom_plan(None);
    crate::file_routes::reset_file_routes();
    crate::file_map::reset_file_map();
    crate::class_map::reset_class_map();
}
