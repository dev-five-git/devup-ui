use std::{
    cmp::Ordering,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

use css::{CounterOwner, Naming, Site, style_origin::Origin, style_selector::StyleSelector};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rustc_hash::FxHashSet;

use super::{
    extract_dynamic_style::ExtractDynamicStyle,
    extract_keyframes::ExtractKeyframes,
    extract_static_style::{ExtractStaticStyle, ThemeTokenResolution},
    numeric_conversion::NumericConversion,
};

#[derive(Default)]
struct RecordingHasher(Vec<Vec<u8>>);

impl Hasher for RecordingHasher {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.push(bytes.to_vec());
    }
}

pub(super) fn trace(value: &impl Hash) -> (Vec<Vec<u8>>, u64) {
    let mut state = RecordingHasher::default();
    value.hash(&mut state);
    let finish = state.finish();
    (state.0, finish)
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct OldStatic<'a> {
    property: &'a str,
    value: &'a str,
    level: u8,
    selector: &'a Option<StyleSelector>,
    order: Option<u8>,
    layer: &'a Option<String>,
    resolution: ThemeTokenResolution,
    naming: Naming,
    owner: CounterOwner,
}

pub(super) fn old_static(style: &ExtractStaticStyle) -> OldStatic<'_> {
    OldStatic {
        property: &style.property,
        value: &style.value,
        level: style.level,
        selector: &style.selector,
        order: style.style_order,
        layer: &style.layer,
        resolution: style.theme_token_resolution,
        naming: style.naming,
        owner: if style.naming == Naming::Own && style.style_order != Some(0) {
            style.counter_owner
        } else {
            CounterOwner::Inactive
        },
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct OldDynamic<'a> {
    property: &'a str,
    level: u8,
    identifier: &'a str,
    selector: Option<&'a StyleSelector>,
    order: Option<u8>,
    important: bool,
    layer: &'a Option<String>,
    naming: Naming,
    site: &'a Option<Site>,
    origin: &'a Origin,
    conversion: NumericConversion,
    presence: bool,
}

pub(super) fn old_dynamic(style: &ExtractDynamicStyle) -> OldDynamic<'_> {
    OldDynamic {
        property: style.property(),
        level: style.level(),
        identifier: style.identifier(),
        selector: style.selector(),
        order: style.style_order(),
        important: style.important(),
        layer: &style.layer,
        naming: style.naming,
        site: &style.site,
        origin: &style.origin,
        conversion: style.conversion(),
        presence: style.presence(),
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct OldKeyframes<'a> {
    keyframes: BTreeMap<String, Vec<OldStatic<'a>>>,
    origin: &'a Origin,
}

pub(super) fn old_keyframes(frames: &ExtractKeyframes) -> OldKeyframes<'_> {
    OldKeyframes {
        keyframes: frames
            .keyframes
            .iter()
            .map(|(step, styles)| (step.clone(), styles.iter().map(old_static).collect()))
            .collect(),
        origin: &frames.origin,
    }
}

pub(super) fn set_lengths<T: Clone + Hash + Ord>(styles: &[T]) -> (usize, usize) {
    (
        styles.iter().cloned().collect::<FxHashSet<_>>().len(),
        styles
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
    )
}

pub(super) fn assert_laws<T: Clone + Hash + Ord + std::fmt::Debug>(values: &[T]) {
    for left in values {
        for right in values {
            let order = left.cmp(right);
            assert_eq!(left.partial_cmp(right), Some(order));
            assert_eq!(left == right, order == Ordering::Equal);
            assert_eq!(order, right.cmp(left).reverse());
            if left == right {
                assert_eq!(trace(left), trace(right));
            }
            for third in values {
                if left <= right && right <= third {
                    assert!(left <= third);
                }
            }
        }
    }
    let mut reversed = values.to_vec();
    reversed.reverse();
    assert_eq!(set_lengths(values), set_lengths(&reversed));
}

pub(super) fn expression<'a>(allocator: &'a Allocator, source: &'a str) -> Expression<'a> {
    let parsed = Parser::new(allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.first() else {
        panic!("fixture expression statement");
    };
    statement.expression.clone_in(allocator)
}
