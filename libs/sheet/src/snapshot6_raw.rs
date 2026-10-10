use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use std::{collections::BTreeSet, fmt};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(super) enum Raw {
    Null,
    Bool(bool),
    Integer(serde_json::Number),
    String(String),
    Array(Vec<Raw>),
    Object(Object),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Object(pub(super) Vec<(String, Raw)>);

impl Serialize for Object {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

struct RawVisitor;
impl<'de> Visitor<'de> for RawVisitor {
    type Value = Raw;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("strict snapshot data")
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Raw, E> {
        Ok(Raw::Null)
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Raw, E> {
        Ok(Raw::Bool(value))
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Raw, E> {
        Ok(Raw::Integer(value.into()))
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Raw, E> {
        Err(E::custom("signed integer is not a snapshot coordinate"))
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Raw, E> {
        Err(E::custom("floating number is not a snapshot coordinate"))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Raw, E> {
        Ok(Raw::String(value.into()))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Raw, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Raw::Array(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Raw, A::Error> {
        let mut keys = BTreeSet::new();
        let mut entries = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Raw>()? {
            if !keys.insert(key.clone()) {
                return Err(serde::de::Error::custom("duplicate decoded object key"));
            }
            entries.push((key, value));
        }
        Ok(Raw::Object(Object(entries)))
    }
}

impl<'de> Deserialize<'de> for Raw {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RawVisitor)
    }
}
