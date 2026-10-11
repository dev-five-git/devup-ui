use super::{names, selected, text};

#[test]
fn retains_computed_destructuring_data_without_inventing_a_static_callable_path() {
    let source = "import {style} from '@vanilla-extract/css';const key='make';const {[key]:make}={make:()=>({padding:8})};const box=style(make());";
    let plan = selected(source);
    assert_eq!(names(&plan.units), vec!["key", "make", "box"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(text(source, plan.roots[0].span), "box=style(make())");
    assert_eq!(plan.helper_calls.len(), 0);
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(text(source, plan.native_calls[0].span), "style(make())");
    assert_eq!(plan.escapes.len(), 0);
}
