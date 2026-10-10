use super::{
    EvidenceError,
    raw::{Object, Raw},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

pub(super) trait Wire: Sized {
    fn read(raw: Raw) -> Result<Self, EvidenceError>;
    fn write(&self) -> Raw;
}

pub(super) trait Key: Ord + Sized {
    fn read_key(raw: String) -> Result<Self, EvidenceError>;
    fn write_key(&self) -> String;
}

impl Key for String {
    fn read_key(raw: String) -> Result<Self, EvidenceError> {
        Ok(raw)
    }
    fn write_key(&self) -> String {
        self.clone()
    }
}

macro_rules! integer {
    ($($ty:ty),*) => { $(
        impl Wire for $ty {
            fn read(raw: Raw) -> Result<Self, EvidenceError> {
                match raw {
                    Raw::Integer(number) => number.as_u64().and_then(|n| <$ty>::try_from(n).ok()).ok_or(EvidenceError::Schema),
                    _ => Err(EvidenceError::Schema),
                }
            }
            fn write(&self) -> Raw { Raw::Integer((*self).into()) }
        }
        impl Key for $ty {
            fn read_key(raw: String) -> Result<Self, EvidenceError> {
                if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) { return Err(EvidenceError::Schema); }
                <$ty>::from_str(&raw).map_err(|_| EvidenceError::Schema)
            }
            fn write_key(&self) -> String { self.to_string() }
        }
    )* };
}
integer!(u8, u32, usize);

impl Wire for String {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match raw {
            Raw::String(value) => Ok(value),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        Raw::String(self.clone())
    }
}
impl Wire for bool {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match raw {
            Raw::Bool(value) => Ok(value),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        Raw::Bool(*self)
    }
}
impl<T: Wire> Wire for Option<T> {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match raw {
            Raw::Null => Ok(None),
            value => T::read(value).map(Some),
        }
    }
    fn write(&self) -> Raw {
        self.as_ref().map_or(Raw::Null, Wire::write)
    }
}
impl<T: Wire> Wire for Vec<T> {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match raw {
            Raw::Array(items) => items.into_iter().map(T::read).collect(),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        Raw::Array(self.iter().map(Wire::write).collect())
    }
}
impl<T: Wire> Wire for Box<T> {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        T::read(raw).map(Box::new)
    }
    fn write(&self) -> Raw {
        self.as_ref().write()
    }
}
impl<A: Wire, B: Wire> Wire for (A, B) {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let mut items = match raw {
            Raw::Array(items) => items.into_iter(),
            _ => return Err(EvidenceError::Schema),
        };
        let a = A::read(items.next().ok_or(EvidenceError::Schema)?)?;
        let b = B::read(items.next().ok_or(EvidenceError::Schema)?)?;
        if items.next().is_some() {
            return Err(EvidenceError::Schema);
        }
        Ok((a, b))
    }
    fn write(&self) -> Raw {
        Raw::Array(vec![self.0.write(), self.1.write()])
    }
}
impl<K: Key, V: Wire> Wire for BTreeMap<K, V> {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let mut map = BTreeMap::new();
        for (key, value) in Object::read(raw)?.0 {
            if map.insert(K::read_key(key)?, V::read(value)?).is_some() {
                return Err(EvidenceError::Schema);
            }
        }
        Ok(map)
    }
    fn write(&self) -> Raw {
        Raw::Object(Object(
            self.iter()
                .map(|(k, v)| (k.write_key(), v.write()))
                .collect(),
        ))
    }
}
impl<T: Wire + Ord> Wire for BTreeSet<T> {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        unique(Vec::<T>::read(raw)?).map(|items| items.into_iter().collect())
    }
    fn write(&self) -> Raw {
        Raw::Array(self.iter().map(Wire::write).collect())
    }
}

impl Object {
    pub(super) fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match raw {
            Raw::Object(value) => Ok(value),
            _ => Err(EvidenceError::Schema),
        }
    }
    pub(super) fn take<T: Wire>(&mut self, key: &str) -> Result<T, EvidenceError> {
        let index = self
            .0
            .iter()
            .position(|(k, _)| k == key)
            .ok_or(EvidenceError::Schema)?;
        T::read(self.0.remove(index).1)
    }
    pub(super) fn finish(self) -> Result<(), EvidenceError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(EvidenceError::Schema)
        }
    }
}

pub(super) fn unique<T: Eq>(items: Vec<T>) -> Result<Vec<T>, EvidenceError> {
    for (index, item) in items.iter().enumerate() {
        if items[..index].contains(item) {
            return Err(EvidenceError::Schema);
        }
    }
    Ok(items)
}

pub(super) fn tagged(name: &str, payload: Raw) -> Raw {
    Raw::Object(Object(vec![(name.into(), payload)]))
}
pub(super) fn variant(raw: Raw) -> Result<(String, Raw), EvidenceError> {
    match raw {
        Raw::String(name) => Ok((name, Raw::Null)),
        Raw::Object(Object(mut entries)) if entries.len() == 1 => {
            entries.pop().ok_or(EvidenceError::Schema)
        }
        _ => Err(EvidenceError::Schema),
    }
}

macro_rules! wire_struct {
    ($ty:path { $($field:ident => $key:literal),* $(,)? }) => {
        impl Wire for $ty {
            fn read(raw: Raw) -> Result<Self, EvidenceError> {
                let mut object = Object::read(raw)?;
                let result = Self { $($field: object.take($key)?,)* };
                object.finish()?;
                Ok(result)
            }
            fn write(&self) -> Raw {
                Raw::Object(Object(vec![$(($key.into(), self.$field.write()),)*]))
            }
        }
    };
}
pub(super) use wire_struct;

macro_rules! wire_named_enum {
    ($ty:path { $($variant:ident { $($field:ident),* $(,)? }),* $(,)? }) => {
        impl Wire for $ty {
            fn read(raw: Raw) -> Result<Self, EvidenceError> {
                let (name, payload) = variant(raw)?;
                let mut object = Object::read(payload)?;
                let value = match name.as_str() {
                    $(stringify!($variant) => Self::$variant { $($field: object.take(stringify!($field))?,)* },)*
                    _ => return Err(EvidenceError::Schema),
                };
                object.finish()?;
                Ok(value)
            }
            fn write(&self) -> Raw {
                match self {
                    $(Self::$variant { $($field),* } => tagged(stringify!($variant), Raw::Object(Object(vec![
                        $((stringify!($field).into(), $field.write()),)*
                    ]))),)*
                }
            }
        }
    };
}
pub(super) use wire_named_enum;
