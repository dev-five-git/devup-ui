use napi::bindgen_prelude::JsObjectValue;
use napi::bindgen_prelude::{Array, Either, FnArgs, FromNapiValue, Function, Uint8Array, Unknown};
use napi::{Env, ValueType};
use serde::de::{self, DeserializeOwned, Error as _, Unexpected, Visitor};
use std::fmt;

/// Construct binding diagnostics in this realm, retaining the raw Error on return.
pub(crate) fn binding_error(env: Env, message: impl AsRef<str>) -> napi::Error {
    let create = || -> napi::Result<napi::Error> {
        let constructor: Function<&str, Unknown<'_>> =
            env.get_global()?.get_named_property("Error")?;
        Ok(napi::Error::from_unknown_without_coercion(
            constructor.new_instance(message.as_ref())?,
        ))
    };
    match create() {
        Ok(error) | Err(error) => error,
    }
}

/// Private control tags; keys and original user values are never encoded here.
pub(crate) enum Operation {
    PreflightFailure,
    Schema,
    Array,
    SafeInt,
    Bytes,
    Debug,
    Bigint,
    Undefined,
    AnyMap,
    Entries,
    Length,
    Index,
    Pair,
    Field,
    Iter,
    Next,
    MapStart,
}
impl From<Operation> for u32 {
    fn from(operation: Operation) -> Self {
        match operation {
            Operation::PreflightFailure => 0,
            Operation::Schema => 1,
            Operation::Array => 2,
            Operation::SafeInt => 3,
            Operation::Bytes => 4,
            Operation::Debug => 5,
            Operation::Bigint => 6,
            Operation::Undefined => 7,
            Operation::AnyMap => 8,
            Operation::Entries => 9,
            Operation::Length => 10,
            Operation::Index => 11,
            Operation::Pair => 12,
            Operation::Field => 13,
            Operation::Iter => 14,
            Operation::Next => 15,
            Operation::MapStart => 16,
        }
    }
}
pub(crate) type AccessCallback<'env> =
    Function<'env, FnArgs<(u32, Unknown<'env>, Option<Either<u32, &'static str>>)>, Unknown<'env>>;
/// Stack-local frame; all handles belong to this synchronous outer entry.
pub(crate) struct Access<'env> {
    pub(crate) env: Env,
    pub(crate) callback: AccessCallback<'env>,
    pub(crate) schema_token: Unknown<'env>,
    pub(crate) failure_token: Unknown<'env>,
}

/// Decoder origin is retained independently of diagnostic text.
#[derive(Debug)]
pub(crate) enum Error {
    Schema(String),
    Access,
    Host(String),
    Transport(napi::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(message) | Self::Host(message) => message.fmt(f),
            Self::Access => f.write_str("JavaScript access failed"),
            Self::Transport(error) => error.reason.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl de::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Schema(message.to_string())
    }
}
impl From<napi::Error> for Error {
    fn from(error: napi::Error) -> Self {
        Self::Transport(error)
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
    slot: u32,
) -> napi::Result<T> {
    T::deserialize(De { value, access }).map_err(|error| match error {
        Error::Schema(_) => schema_error(value, access, slot),
        Error::Access | Error::Host(_) => binding_error(access.env, format!("Error: {error}")),
        Error::Transport(error) => error,
    })
}
/// Preflight forgives schema errors and converted caught imports, never raw access faults.
pub(crate) fn decode_preflight<'env, T: DeserializeOwned + Default>(
    value: Unknown<'env>,
    access: &Access<'env>,
) -> napi::Result<T> {
    match T::deserialize(De { value, access }) {
        Ok(value) => Ok(value),
        Err(Error::Schema(_)) => Ok(T::default()),
        Err(Error::Access) => {
            De { value, access }
                .op::<Unknown<'env>>(Operation::PreflightFailure, None)
                .map_err(|error| match error {
                    Error::Transport(error) => error,
                    error @ (Error::Schema(_) | Error::Access | Error::Host(_)) => {
                        binding_error(access.env, format!("Error: {error}"))
                    }
                })?;
            Ok(T::default())
        }
        Err(error @ Error::Host(_)) => Err(binding_error(access.env, format!("Error: {error}"))),
        Err(Error::Transport(error)) => Err(error),
    }
}
/// Notify only on failure; the JS facade replays access failures before this token.
pub(crate) fn schema_error<'env>(
    value: Unknown<'env>,
    access: &Access<'env>,
    slot: u32,
) -> napi::Error {
    match access
        .callback
        .call((u32::from(Operation::Schema), value, Some(Either::A(slot))).into())
    {
        Ok(_) => binding_error(access.env, "invalid argument"),
        Err(error) => error,
    }
}
/// Read only a fresh private metadata packet, never properties of an original input.
pub(crate) fn item<T: FromNapiValue>(array: &Array<'_>, index: u32) -> Result<T, Error> {
    array
        .get(index)?
        .ok_or_else(|| Error::Host("missing private access result".to_string()))
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
    pub(crate) fn op<T: FromNapiValue>(
        self,
        operation: Operation,
        key: Option<Either<u32, &'static str>>,
    ) -> Result<T, Error> {
        let result = self
            .access
            .callback
            .call((u32::from(operation), self.value, key).into())?;
        if result.get_type()? == ValueType::Function {
            if self
                .access
                .env
                .strict_equals(result, self.access.schema_token)?
            {
                return Err(Error::custom("invalid intrinsic receiver"));
            }
            if self
                .access
                .env
                .strict_equals(result, self.access.failure_token)?
            {
                return Err(Error::Access);
            }
        }
        Ok(T::from_unknown(result)?)
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
            _ => match self.op::<Option<Uint8Array>>(Operation::Bytes, None)? {
                Some(bytes) => Error::invalid_type(Unexpected::Bytes(bytes.as_ref()), expected),
                None => Error::invalid_type(
                    Unexpected::Other(&self.op::<String>(Operation::Debug, None)?),
                    expected,
                ),
            },
        })
    }
    pub(crate) fn unsigned<'de, V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.op::<bool>(Operation::SafeInt, None)? {
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
        self.op::<String>(Operation::Bigint, None)?
            .parse()
            .map_err(|_| {
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
