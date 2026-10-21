use napi::ValueType;
use napi::bindgen_prelude::{Array, FnArgs, FromNapiValue, Function, Unknown};
use serde::de::{self, DeserializeOwned, Error as _, Unexpected, Visitor};
use std::fmt;

/// Scoped access/exception operations supplied only by the private JS facade.
pub(crate) type Access<'env> =
    Function<'env, FnArgs<(String, Unknown<'env>, Option<String>)>, Array<'env>>;

/// A Serde diagnostic, before the WASM-compatible outer Error is constructed.
#[derive(Debug)]
pub(crate) struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}
impl de::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}
impl From<napi::Error> for Error {
    fn from(error: napi::Error) -> Self {
        Self(error.reason)
    }
}
/// One original JS value; children retain the same scoped decoder and access seam.
#[derive(Clone, Copy)]
pub(crate) struct De<'a, 'env> {
    pub(crate) value: Unknown<'env>,
    pub(crate) access: &'a Access<'env>,
}
/// Decode without preconversion, retaining existing engine Deserialize implementations.
pub(crate) fn decode<'env, T: DeserializeOwned>(
    value: Unknown<'env>,
    access: &Access<'env>,
) -> napi::Result<T> {
    T::deserialize(De { value, access })
        .map_err(|error| napi::Error::from_reason(format!("Error: {error}")))
}
/// Read only a fresh private control envelope, never properties of an original input.
pub(crate) fn item<T: FromNapiValue>(array: &Array<'_>, index: u32) -> Result<T, Error> {
    array
        .get(index)?
        .ok_or_else(|| Error::custom("missing private access result"))
}
impl<'env> De<'_, 'env> {
    pub(crate) const fn child(self, value: Unknown<'env>) -> Self {
        Self { value, ..self }
    }
    pub(crate) fn kind(self) -> Result<ValueType, Error> {
        Ok(self.value.get_type()?)
    }
    pub(crate) fn nullish(self) -> Result<bool, Error> {
        Ok(matches!(
            self.kind()?,
            ValueType::Null | ValueType::Undefined
        ))
    }
    pub(crate) fn op<T: FromNapiValue>(self, name: &str, key: Option<String>) -> Result<T, Error> {
        let result = self
            .access
            .call((name.to_string(), self.value, key).into())?;
        if !item::<bool>(&result, 0)? {
            return Err(Error::custom("JavaScript access failed"));
        }
        item(&result, 1)
    }
    pub(crate) fn invalid(self, expected: &dyn de::Expected) -> Result<Error, Error> {
        Ok(match self.kind()? {
            ValueType::Null | ValueType::Undefined => {
                Error::invalid_type(Unexpected::Unit, expected)
            }
            ValueType::Boolean => {
                Error::invalid_type(Unexpected::Bool(bool::from_unknown(self.value)?), expected)
            }
            ValueType::Number => {
                Error::invalid_type(Unexpected::Float(f64::from_unknown(self.value)?), expected)
            }
            ValueType::String => Error::invalid_type(
                Unexpected::Str(&String::from_unknown(self.value)?),
                expected,
            ),
            _ => match self.op::<Option<Vec<u8>>>("bytes", None)? {
                Some(bytes) => Error::invalid_type(Unexpected::Bytes(&bytes), expected),
                None => Error::invalid_type(
                    Unexpected::Other(&self.op::<String>("debug", None)?),
                    expected,
                ),
            },
        })
    }
    pub(crate) fn unsigned<'de, V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.op::<bool>("safeInt", None)? {
            let number = i64::from_unknown(self.value)?;
            if let Ok(number) = u64::try_from(number) {
                return visitor.visit_u64(number);
            }
        }
        Err(self.invalid(&visitor)?)
    }
    pub(crate) fn bigint<T: std::str::FromStr>(
        self,
        target: &str,
        bounds: &str,
    ) -> Result<T, Error> {
        self.op::<String>("bigint", None)?.parse().map_err(|_| {
            Error::custom(format!(
                "Couldn't deserialize {target} from a BigInt outside {bounds} bounds"
            ))
        })
    }
}
/// Widen on approved native hosts, whose usize byte width is always eight.
pub(crate) fn widen_index(value: u32) -> usize {
    usize::from_le_bytes(u64::from(value).to_le_bytes())
}
/// A serialized usize is wasm32-sized on every native host, with usize diagnostics.
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Wasm32Index(pub(crate) usize);
impl<'de> serde::Deserialize<'de> for Wasm32Index {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IndexVisitor;
        impl<'value> Visitor<'value> for IndexVisitor {
            type Value = Wasm32Index;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("usize")
            }
            fn visit_newtype_struct<D: serde::Deserializer<'value>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_u64(self)
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                let value = u32::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))?;
                Ok(Wasm32Index(widen_index(value)))
            }
        }
        deserializer.deserialize_newtype_struct("Wasm32Index", IndexVisitor)
    }
}
