use std::{
    cmp::Ordering,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

use css::style_origin::Origin;

use super::ExtractKeyframes;
use crate::extract_style::{ProducerPolicy, extract_static_style::ExtractStaticStyle};

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Identity<'a> {
    keyframes: &'a BTreeMap<String, Vec<ExtractStaticStyle>>,
    origin: &'a Origin,
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CounterIdentity<'a> {
    keyframes: &'a BTreeMap<String, Vec<ExtractStaticStyle>>,
    original: u32,
}

impl ExtractKeyframes {
    const fn identity(&self) -> Identity<'_> {
        Identity {
            keyframes: &self.keyframes,
            origin: &self.origin,
        }
    }

    const fn counter_identity(&self, original: u32) -> CounterIdentity<'_> {
        CounterIdentity {
            keyframes: &self.keyframes,
            original,
        }
    }
}

impl PartialEq for ExtractKeyframes {
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
impl Eq for ExtractKeyframes {}
impl Hash for ExtractKeyframes {
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
impl PartialOrd for ExtractKeyframes {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ExtractKeyframes {
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
