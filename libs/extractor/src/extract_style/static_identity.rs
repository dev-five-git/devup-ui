use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use css::{Naming, naming::CounterOwner, style_selector::StyleSelector};

use super::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Identity<'a> {
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

impl ExtractStaticStyle {
    fn identity(&self) -> Identity<'_> {
        Identity {
            property: &self.property,
            value: &self.value,
            level: self.level,
            selector: &self.selector,
            order: self.style_order,
            layer: &self.layer,
            resolution: self.theme_token_resolution,
            naming: self.naming,
            owner: if self.naming == Naming::Own && self.style_order != Some(0) {
                self.counter_owner
            } else {
                CounterOwner::Inactive
            },
        }
    }
}

impl PartialEq for ExtractStaticStyle {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}
impl Eq for ExtractStaticStyle {}
impl Hash for ExtractStaticStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity().hash(state);
    }
}
impl PartialOrd for ExtractStaticStyle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ExtractStaticStyle {
    fn cmp(&self, other: &Self) -> Ordering {
        self.identity().cmp(&other.identity())
    }
}
