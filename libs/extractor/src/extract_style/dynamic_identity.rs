use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use css::{Naming, Site, style_origin::Origin, style_selector::StyleSelector};

use super::ExtractDynamicStyle;
use crate::extract_style::ProducerPolicy;
use crate::extract_style::counter_selector::CounterSelector;

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Identity<'a> {
    property: &'a str,
    level: u8,
    identifier: &'a str,
    selector: &'a Option<StyleSelector>,
    order: Option<u8>,
    important: bool,
    layer: &'a Option<String>,
    naming: Naming,
    site: &'a Option<Site>,
    origin: &'a Origin,
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CounterIdentity<'a> {
    property: &'a str,
    level: u8,
    identifier: &'a str,
    selector: CounterSelector<'a>,
    order: Option<u8>,
    important: bool,
    layer: &'a Option<String>,
    site: &'a Option<Site>,
    owner: Option<u32>,
}

impl ExtractDynamicStyle {
    fn identity(&self) -> Identity<'_> {
        Identity {
            property: &self.property,
            level: self.level,
            identifier: &self.identifier,
            selector: &self.selector,
            order: self.style_order,
            important: self.important,
            layer: &self.layer,
            naming: self.naming,
            site: &self.site,
            origin: &self.origin,
        }
    }

    fn counter_identity(&self, original: u32) -> CounterIdentity<'_> {
        CounterIdentity {
            property: &self.property,
            level: self.level,
            identifier: &self.identifier,
            selector: CounterSelector(&self.selector),
            order: self.style_order,
            important: self.important,
            layer: &self.layer,
            site: &self.site,
            owner: ProducerPolicy::declaration_owner(original, self.style_order),
        }
    }
}

impl PartialEq for ExtractDynamicStyle {
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
impl Eq for ExtractDynamicStyle {}
impl Hash for ExtractDynamicStyle {
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
impl PartialOrd for ExtractDynamicStyle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ExtractDynamicStyle {
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
