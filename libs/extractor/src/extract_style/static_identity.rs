use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use css::{CounterOwner, Naming, style_selector::StyleSelector};

use super::ProducerPolicy;
use super::counter_selector::CounterSelector;
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

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CounterIdentity<'a> {
    property: &'a str,
    value: &'a str,
    level: u8,
    selector: CounterSelector<'a>,
    order: Option<u8>,
    layer: &'a Option<String>,
    resolution: ThemeTokenResolution,
    owner: Option<u32>,
}

impl ExtractStaticStyle {
    fn counter_identity(&self, original: u32) -> CounterIdentity<'_> {
        CounterIdentity {
            property: &self.property,
            value: &self.value,
            level: self.level,
            selector: CounterSelector(&self.selector),
            order: self.style_order,
            layer: &self.layer,
            resolution: self.theme_token_resolution,
            owner: ProducerPolicy::declaration_owner(original, self.style_order),
        }
    }

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
        match (self.producer_policy(), other.producer_policy()) {
            (ProducerPolicy::Current, ProducerPolicy::Current) => {
                self.identity() == other.identity()
            }
            (ProducerPolicy::Current, ProducerPolicy::CounterOriginal(_))
            | (ProducerPolicy::CounterOriginal(_), ProducerPolicy::Current) => false,
            (ProducerPolicy::CounterOriginal(left), ProducerPolicy::CounterOriginal(right)) => {
                self.counter_identity(left) == other.counter_identity(right)
            }
        }
    }
}
impl Eq for ExtractStaticStyle {}
impl Hash for ExtractStaticStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.producer_policy() {
            ProducerPolicy::Current => self.identity().hash(state),
            ProducerPolicy::CounterOriginal(original) => {
                1u8.hash(state);
                self.counter_identity(original).hash(state);
            }
        }
    }
}
impl PartialOrd for ExtractStaticStyle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ExtractStaticStyle {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.producer_policy(), other.producer_policy()) {
            (ProducerPolicy::Current, ProducerPolicy::Current) => {
                self.identity().cmp(&other.identity())
            }
            (ProducerPolicy::Current, ProducerPolicy::CounterOriginal(_)) => Ordering::Less,
            (ProducerPolicy::CounterOriginal(_), ProducerPolicy::Current) => Ordering::Greater,
            (ProducerPolicy::CounterOriginal(left), ProducerPolicy::CounterOriginal(right)) => self
                .counter_identity(left)
                .cmp(&other.counter_identity(right)),
        }
    }
}
