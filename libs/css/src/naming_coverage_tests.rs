use crate::style_selector::{AtRule, AtRuleKind, StyleSelector};
use crate::{Naming, atom_name, content_name::AtomContent};
use serial_test::serial;
use std::collections::{BTreeSet, HashMap};

struct Configuration;

impl Drop for Configuration {
    fn drop(&mut self) {
        crate::atom_hoist::set_atom_hoist(None);
        crate::atom_hoist::restore_atom_plan(None);
        crate::file_map::reset_canonical_map();
        crate::set_prefix(None);
        crate::debug::set_debug(false);
    }
}

#[test]
#[serial]
fn placement_scope_uses_canonical_buckets_not_original_cleanup_owners() {
    // Given: distinct originals delivering into local and hoisted canonical buckets.
    let _configuration = Configuration;
    crate::atom_hoist::set_atom_hoist(Some(2));
    crate::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["shared.tsx".into()])));
    crate::file_map::set_canonical_map(HashMap::from([
        ("child.tsx".into(), "a.tsx".into()),
        ("shared-child.tsx".into(), "shared.tsx".into()),
    ]));
    // When: the public pure placement helper resolves each scope.
    let scopes = [
        None,
        Some("child.tsx"),
        Some("a.tsx"),
        Some("shared-child.tsx"),
    ]
    .map(atom_name::scope);
    // Then: local aliases coincide, but global and hoisted delivery stay separate.
    assert_eq!(scopes, ["g", "l-612e747378", "l-612e747378", "h"]);
}

#[test]
fn nested_layer_selector_identity_keeps_inner_selector_but_not_cleanup_file() {
    // Given: equal layer structure with a pseudo selector, and one different selector.
    let selector = |inner: &str, file: &str| StyleSelector::At {
        kind: AtRuleKind::Layer,
        query: "cards".into(),
        selector: Some(inner.into()),
        outer: vec![AtRule {
            kind: AtRuleKind::Layer,
            query: "base".into(),
        }],
        file: Some(file.into()),
    };
    let a = selector("&:hover", "a.tsx");
    let b = selector("&:hover", "b.tsx");
    let focus = selector("&:focus", "a.tsx");
    let content = |selector| {
        AtomContent {
            property: "color",
            value: Some("red"),
            naming: Naming::Own,
            level: 0,
            order: 255,
            selector: Some(selector),
            layer: None,
            dynamic: false,
        }
        .content()
    };
    // When: lossless selector keys and versioned descriptors are built from the same structure.
    let keys = [&a, &b, &focus].map(|value| atom_name::selector_key(Some(value), None));
    let names = [&a, &b, &focus].map(content);
    // Then: cleanup does not rename a rule, while a changed pseudo-selector must.
    assert_eq!(keys[0], keys[1]);
    assert_ne!(keys[0], keys[2]);
    assert_eq!(
        keys[0],
        "a-layer-62617365-end-layer-6361726473-s-263a686f766572-n"
    );
    assert_eq!(names[0], names[1]);
    assert_ne!(names[0].descriptor, names[2].descriptor);
    assert_ne!(names[0].name(""), names[2].name(""));
    assert!(names[0].lossless.contains("-a3base-e-3cards-s_a_nhover"));
}

#[test]
#[serial]
fn pure_keyframe_api_respects_atom_scope_and_legacy_content_controls() {
    // Given: a custom prefix and canonical delivery alias in atom mode.
    let _configuration = Configuration;
    crate::set_prefix(Some("du-".into()));
    crate::debug::set_debug(false);
    crate::atom_hoist::set_atom_hoist(Some(2));
    crate::file_map::set_canonical_map(HashMap::from([("child.tsx".into(), "a.tsx".into())]));
    // When: the existing low-level API names global/local animations, then its legacy control.
    let global = crate::keyframes_to_keyframes_name("spin", None);
    let child = crate::keyframes_to_keyframes_name("spin", Some("child.tsx"));
    let canonical = crate::keyframes_to_keyframes_name("spin", Some("a.tsx"));
    let other = crate::keyframes_to_keyframes_name("spin", Some("b.tsx"));
    crate::atom_hoist::set_atom_hoist(None);
    let legacy = crate::keyframes_to_keyframes_name("spin", Some("child.tsx"));
    // Then: atom placement is canonical and injective, legacy stays content-only.
    assert_eq!(global, "du-k1-g-7370696e");
    assert_eq!(child, "du-k1-l-612e747378-7370696e");
    assert_eq!(child, canonical);
    assert_ne!(child, other);
    assert_ne!(global, child);
    assert_eq!(legacy, "du-Kspin");
}
