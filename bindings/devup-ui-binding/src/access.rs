use crate::value::{De, Error, item};
use napi::ValueType;
use napi::bindgen_prelude::{Array, Unknown};
use serde::Deserializer as _;
use serde::de::{self, DeserializeSeed, Error as _, IntoDeserializer, Visitor};

/// A snapshot-length array traversal or an owned scoped JS iterator traversal.
pub(crate) struct Sequence<'a, 'env> {
    source: De<'a, 'env>,
    range: Option<std::ops::Range<u32>>,
}
impl<'a, 'env> Sequence<'a, 'env> {
    pub(crate) fn new(source: De<'a, 'env>, pairs: bool) -> Result<Option<Self>, Error> {
        if !pairs && source.op::<bool>("array", None)? {
            return Self::array(source).map(Some);
        }
        let iterator: Unknown<'env> = source.op("iter", None)?;
        if iterator.get_type()? != ValueType::Undefined {
            return Ok(Some(Self {
                source: source.child(iterator),
                range: None,
            }));
        }
        if pairs && source.kind()? == ValueType::Object {
            let (entries, len): (Unknown<'env>, u32) = source.op("entries", None)?;
            return Ok(Some(Self {
                source: source.child(entries),
                range: Some(0..len),
            }));
        }
        Ok(None)
    }
    fn array(source: De<'a, 'env>) -> Result<Self, Error> {
        Ok(Self {
            range: Some(0..source.op("length", None)?),
            source,
        })
    }
    fn next(&mut self) -> Result<Option<De<'a, 'env>>, Error> {
        if let Some(range) = &mut self.range {
            range
                .next()
                .map(|index| {
                    self.source
                        .op("index", Some(index.to_string()))
                        .map(|value| self.source.child(value))
                })
                .transpose()
        } else {
            let result: Array<'env> = self.source.op("next", None)?;
            if item::<bool>(&result, 0)? {
                Ok(None)
            } else {
                Ok(Some(self.source.child(item(&result, 1)?)))
            }
        }
    }
}
impl<'de> de::SeqAccess<'de> for Sequence<'_, '_> {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        self.next()?
            .map(|value| seed.deserialize(value))
            .transpose()
    }
    fn size_hint(&self) -> Option<usize> {
        self.range.as_ref().map(std::iter::ExactSizeIterator::len)
    }
}
enum Keys<'a, 'env> {
    Pairs(Sequence<'a, 'env>),
    Fields(De<'a, 'env>, std::slice::Iter<'static, &'static str>),
}
/// Distinct pair-stream and declared-field map access, retaining a read-once value.
pub(crate) struct Entries<'a, 'env> {
    keys: Keys<'a, 'env>,
    value: De<'a, 'env>,
}
impl<'a, 'env> Entries<'a, 'env> {
    pub(crate) const fn pairs(sequence: Sequence<'a, 'env>) -> Self {
        let value = sequence.source;
        Self {
            keys: Keys::Pairs(sequence),
            value,
        }
    }
    pub(crate) fn fields(source: De<'a, 'env>, fields: &'static [&'static str]) -> Self {
        Self {
            keys: Keys::Fields(source, fields.iter()),
            value: source,
        }
    }
}
impl<'de> de::MapAccess<'de> for Entries<'_, '_> {
    type Error = Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match &mut self.keys {
            Keys::Pairs(sequence) => match sequence.next()? {
                Some(pair) => {
                    let pair_values: Array<'_> = pair.op("pair", None)?;
                    self.value = pair.child(item(&pair_values, 1)?);
                    seed.deserialize(pair.child(item(&pair_values, 0)?))
                        .map(Some)
                }
                None => Ok(None),
            },
            Keys::Fields(source, fields) => {
                for field in fields {
                    let result: Array<'_> = source.op("field", Some((*field).to_string()))?;
                    if item::<bool>(&result, 0)? {
                        self.value = source.child(item(&result, 1)?);
                        return seed.deserialize((*field).into_deserializer()).map(Some);
                    }
                }
                Ok(None)
            }
        }
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        seed.deserialize(self.value)
    }
}
/// Externally tagged enums use Object.entries, never iterable-map dispatch.
pub(crate) struct Variant<'a, 'env> {
    tag: De<'a, 'env>,
    payload: De<'a, 'env>,
}
impl<'a, 'env> Variant<'a, 'env> {
    pub(crate) fn new(source: De<'a, 'env>, expected: &dyn de::Expected) -> Result<Self, Error> {
        match source.kind()? {
            ValueType::String => Ok(Self {
                tag: source,
                payload: source.child(source.op("undefined", None)?),
            }),
            ValueType::Object => {
                let (entries, len): (Unknown<'env>, u32) = source.op("entries", None)?;
                let entries = source.child(entries);
                if len != 1 {
                    let len: u32 = entries.op("length", None)?;
                    return Err(Error::invalid_length(
                        usize::try_from(len).map_err(Error::custom)?,
                        &"1",
                    ));
                }
                let pair = entries.child(entries.op("index", Some("0".to_string()))?);
                let values: Array<'env> = pair.op("pair", None)?;
                Ok(Self {
                    tag: source.child(item(&values, 0)?),
                    payload: source.child(item(&values, 1)?),
                })
            }
            _ => Err(source.invalid(expected)?),
        }
    }
}
impl<'de, 'a, 'env> de::EnumAccess<'de> for Variant<'a, 'env> {
    type Error = Error;
    type Variant = De<'a, 'env>;
    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Error> {
        Ok((seed.deserialize(self.tag)?, self.payload))
    }
}
impl<'de> de::VariantAccess<'de> for De<'_, '_> {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        serde::Deserialize::deserialize(self)
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(self)
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_tuple(len, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_struct("", fields, visitor)
    }
}
