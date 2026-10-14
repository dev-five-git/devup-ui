use super::{ProducedAllocation, extract_static_style::ExtractStaticStyle};
use crate::sparse_sites::SiteScope;
use css::{allocation_input::NameMode, counter_names::NameAddress};

pub(super) fn reset(mode: NameMode) {
    css::set_prefix(Some("p".to_string()));
    css::debug::set_debug(mode == NameMode::Debug);
    css::atom_hoist::set_atom_hoist((mode == NameMode::AtomHoist).then_some(2));
    css::atom_hoist::restore_atom_plan(None);
    css::file_map::reset_canonical_map();
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    css::theme_tokens::set_theme_token_values(Default::default(), Default::default());
}

pub(super) fn scope(filename: &str) -> SiteScope {
    let original = css::file_map::get_or_insert_original_id(filename)
        .unwrap_or_else(|error| panic!("{error}"));
    SiteScope::enter_counter_numbered(original, "abcdefghij", &[])
}

pub(super) fn cleanup() {
    reset(NameMode::Counter);
    css::set_prefix(None);
}

pub(super) fn static_style(filename: &str) -> ExtractStaticStyle {
    let _scope = scope(filename);
    ExtractStaticStyle::new("color", "red", 0, None)
}

pub(super) fn address(receipt: &ProducedAllocation, namespace: &str, key: &str, slot: usize) {
    assert_eq!(
        receipt.allocation.address,
        NameAddress::Counter {
            namespace: namespace.to_string(),
            legacy_key: key.to_string(),
            slot,
        }
    );
}

pub(super) fn produced<T>(value: Result<T, super::CounterProducerError>) -> T {
    value.unwrap_or_else(|error| panic!("{error}"))
}
