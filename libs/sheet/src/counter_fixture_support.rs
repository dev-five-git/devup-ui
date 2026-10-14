use css::counter_names::NameAddress;
use extractor::{
    counter_test_support::{FixtureRequest, with_original},
    extract_style::ProducedAllocation,
};

struct Restore<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for Restore<F> {
    fn drop(&mut self) {
        if let Some(restore) = self.0.take() {
            restore();
        }
    }
}

pub(super) fn state() -> impl Drop {
    let prefix = css::get_prefix();
    let debug = css::debug::is_debug();
    let threshold = css::atom_hoist::atom_hoist_threshold();
    let plan = css::atom_hoist::atom_plan();
    let classes = css::class_map::get_class_map();
    let files = css::file_map::get_file_map();
    let originals = css::file_map::get_original_ids();
    let canonical = css::file_map::get_canonical_map();
    let restore = Restore(Some(move || {
        css::set_prefix(prefix);
        css::debug::set_debug(debug);
        css::atom_hoist::set_atom_hoist(threshold);
        css::atom_hoist::restore_atom_plan(plan);
        css::class_map::set_class_map(classes);
        css::file_map::set_file_map(files);
        css::file_map::set_original_ids(originals);
        css::file_map::set_canonical_map(canonical);
    }));
    css::set_prefix(Some("p".into()));
    css::debug::set_debug(false);
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::class_map::reset_class_map();
    restore
}

pub(super) fn fixture<T>(filename: &str, build: impl FnOnce() -> T) -> T {
    with_original(
        FixtureRequest {
            filename,
            source: "a\nbcd",
        },
        build,
    )
    .unwrap_or_else(|error| panic!("constructor fixture: {error}"))
}

pub(super) fn produced<T>(result: Result<T, extractor::extract_style::CounterProducerError>) -> T {
    result.unwrap_or_else(|error| panic!("authentic producer: {error}"))
}

pub(super) fn address(receipt: &ProducedAllocation, expected: (&str, &str, usize, &str)) {
    let (namespace, key, slot, name) = expected;
    assert_eq!(
        receipt.allocation.address,
        NameAddress::Counter {
            namespace: namespace.into(),
            legacy_key: key.into(),
            slot,
        }
    );
    assert_eq!(receipt.allocation.name, name);
}
