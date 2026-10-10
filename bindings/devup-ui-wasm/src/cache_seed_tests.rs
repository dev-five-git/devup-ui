use super::*;

fn seed_batches() {
    seed_file_map(vec!["b.tsx".into(), "b.tsx".into()]);
    seed_file_map(vec!["a.tsx".into(), "b.tsx".into()]);
    seed_file_map(vec!["c.tsx".into(), "a.tsx".into(), "c.tsx".into()]);
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn rollback_keeps_seed_call_order_and_repeated_files(#[case] class_companion: bool) {
    // Given: independently cold incremental numbering and an admitted empty cache.
    const SOURCE: &str =
        "import {Box} from '@devup-ui/react';export const x=(p)=><Box color='red' w={p.width}/>;";
    fresh();
    seed_batches();
    let cold = output("a.tsx", SOURCE, false);
    let cold_ids = css::file_map::get_original_ids();
    let cold_files = css::file_map::get_file_map();
    fresh();
    let empty = snapshot();
    assert_eq!(import(empty), Ok(()));
    seed_batches();
    // When: an invalid companion rolls back cached state after three fresh seed calls.
    if class_companion {
        cache_restore::classes(None);
    } else {
        cache_restore::files(None);
    }
    // Then: original/delivery numbering and the full emitted output match those same cold calls.
    assert_eq!(css::file_map::get_original_ids(), cold_ids);
    assert_eq!(css::file_map::get_file_map(), cold_files);
    assert_eq!(output("a.tsx", SOURCE, false), cold);
    fresh();
}
