/// Invalidate imported evaluation results at a quiescent production state boundary.
/// Exact-source stripping and active evaluation/scratch stacks are unaffected.
pub fn invalidate_evaluation_cache() {
    crate::vanilla_extract::discard_reserved_references();
}
