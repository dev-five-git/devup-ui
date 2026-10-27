use crate::access::{Entries, Sequence, Variant};
use crate::value::{De, Error};
use napi::ValueType;
use napi::bindgen_prelude::FromNapiValue;
use serde::de::{self, Error as _, Visitor};

macro_rules! integers {
    ($method:ident: $($name:ident),+) => {$ (
        fn $name<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            self.$method(visitor)
        }
    )+};
}
impl<'de> de::Deserializer<'de> for De<'_, '_> {
    type Error = Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.kind()? {
            ValueType::Null | ValueType::Undefined => visitor.visit_unit(),
            ValueType::Boolean => visitor.visit_bool(bool::from_unknown(self.value)?),
            ValueType::Number => {
                if self.op::<bool>("safeInt", None)? {
                    visitor.visit_i64(i64::from_unknown(self.value)?)
                } else {
                    visitor.visit_f64(f64::from_unknown(self.value)?)
                }
            }
            ValueType::BigInt => {
                let text: String = self.op("bigint", None)?;
                match text.parse::<i64>() {
                    Ok(value) => visitor.visit_i64(value),
                    Err(_) => visitor.visit_u64(text.parse::<u64>().map_err(|_| Error::custom(
                        "Couldn't deserialize i64 or u64 from a BigInt outside i64::MIN..u64::MAX bounds"))?),
                }
            }
            ValueType::String => visitor.visit_string(String::from_unknown(self.value)?),
            ValueType::Object => {
                if self.op::<bool>("array", None)? {
                    return self.deserialize_seq(visitor);
                }
                if let Some(bytes) = self.op::<Option<Vec<u8>>>("bytes", None)? {
                    return visitor.visit_byte_buf(bytes);
                }
                if self.op::<bool>("anyMap", None)? {
                    return self.deserialize_map(visitor);
                }
                Err(self.invalid(&visitor)?)
            }
            _ => Err(self.invalid(&visitor)?),
        }
    }
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.kind()? == ValueType::Boolean {
            visitor.visit_bool(bool::from_unknown(self.value)?)
        } else {
            Err(self.invalid(&visitor)?)
        }
    }
    integers!(unsigned: deserialize_u16, deserialize_u32);
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.kind()? == ValueType::BigInt {
            visitor.visit_u64(self.bigint("u64", "u64::MIN..u64::MAX")?)
        } else {
            self.unsigned(visitor)
        }
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.kind()? == ValueType::String {
            visitor.visit_string(String::from_unknown(self.value)?)
        } else {
            Err(self.invalid(&visitor)?)
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.nullish()? {
            self.deserialize_ignored_any(visitor)
        } else {
            Err(self.invalid(&visitor)?)
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.nullish()? {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match Sequence::new(self, false)? {
            Some(sequence) => visitor.visit_seq(sequence),
            None => Err(self.invalid(&visitor)?),
        }
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match Sequence::new(self, true)? {
            Some(sequence) => visitor.visit_map(Entries::pairs(sequence)),
            None => Err(self.invalid(&visitor)?),
        }
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        if self.kind()? == ValueType::Object {
            visitor.visit_map(Entries::fields(self, fields))
        } else {
            Err(self.invalid(&visitor)?)
        }
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let variant = Variant::new(self, &visitor)?;
        visitor.visit_enum(variant)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }
    serde::forward_to_deserialize_any! {
        char i8 i16 i32 i64 i128 u8 u128 f32 f64 str bytes byte_buf unit_struct tuple_struct
    }
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_string(visitor)
    }
    fn deserialize_tuple<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }
}
