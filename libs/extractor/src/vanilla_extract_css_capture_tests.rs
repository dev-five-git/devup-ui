use super::{Values, validate};
use crate::{own_css_plan::Plan, vanilla_extract::Collector};
use boa_engine::{Context, Source};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{cell::RefCell, rc::Rc};

#[test]
fn capture_keeps_numeric_data_keys_and_scalar_declarations() -> Result<(), String> {
    // Given: a numeric key belongs to ignored props, not a CSS declaration.
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan::default(),
    };
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes("({props:{0:'data'},color:'red'})"))
        .map_err(|error| error.to_string())?;
    // When
    let root = values
        .capture(&value, &mut context)
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(
        values.plan.code(root),
        r#"{"props":{"0":"data"},"color":"red"}"#
    );
    assert_eq!(validate(&values.plan, root), vec![]);
    Ok(())
}

#[test]
fn capture_rejects_nested_responsive_arrays_before_lowering() -> Result<(), String> {
    // Given
    let names = FxHashMap::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut values = Values {
        names: &names,
        collector: &collector,
        active: FxHashSet::default(),
        key_slots: FxHashMap::default(),
        plan: Plan::default(),
    };
    let mut context = Context::default();
    let value = context
        .eval(Source::from_bytes("({p:[[1,2]]})"))
        .map_err(|error| error.to_string())?;
    // When
    let error = values
        .capture(&value, &mut context)
        .err()
        .ok_or("nested array accepted")?;
    // Then
    assert!(error.to_string().contains(crate::utils::RESPONSIVE_ARRAY));
    Ok(())
}
