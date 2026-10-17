use super::*;

#[test]
fn source_type_returns_no_constants_when_an_unvalidated_internal_path_is_unknown() {
    let option = ExtractOption::default();
    let mut modules = Modules {
        resolver: None,
        option: &option,
        exports: FxHashMap::default(),
        loading: Vec::new(),
    };
    assert_eq!(
        modules
            .read("token.custom", "export const PRIMARY = 'red';", None)
            .len(),
        0
    );
}
