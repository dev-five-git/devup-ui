use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Undefined,
    Null,
    Bool(bool),
    Text(String),
    Object(Rc<RefCell<BTreeMap<String, Value>>>),
    Array(Rc<RefCell<Vec<Value>>>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Null, Self::Null) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => Rc::ptr_eq(a, b),
            (Self::Array(a), Self::Array(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Value {
    pub fn object(fields: &[(&str, Self)]) -> Self {
        Self::Object(Rc::new(RefCell::new(
            fields
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
        )))
    }

    pub fn text(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_) | Self::Array(_))
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text.clone()),
            Self::Undefined | Self::Null | Self::Bool(_) | Self::Object(_) | Self::Array(_) => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            Self::Undefined | Self::Null | Self::Text(_) | Self::Object(_) | Self::Array(_) => None,
        }
    }

    pub fn get(&self, key: &str) -> Result<Self, Self> {
        match self {
            Self::Object(fields) => {
                Ok(fields.borrow().get(key).cloned().unwrap_or(Self::Undefined))
            }
            Self::Array(values) => Ok(key
                .parse::<usize>()
                .ok()
                .and_then(|index| values.borrow().get(index).cloned())
                .unwrap_or(Self::Undefined)),
            Self::Undefined | Self::Null | Self::Bool(_) | Self::Text(_) => {
                Err(Self::text("native Reflect.get primitive target"))
            }
        }
    }

    pub fn set(&self, key: &str, value: Self) {
        let Self::Object(fields) = self else {
            panic!("native property assignment needs an object")
        };
        fields.borrow_mut().insert(key.to_owned(), value);
    }

    pub(crate) fn json_value(&self) -> Option<serde_json::Value> {
        match self {
            Self::Undefined => None,
            Self::Null => Some(serde_json::Value::Null),
            Self::Bool(value) => Some((*value).into()),
            Self::Text(value) => Some(value.clone().into()),
            Self::Object(fields) => Some(serde_json::Value::Object(
                fields
                    .borrow()
                    .iter()
                    .filter_map(|(key, value)| value.json_value().map(|value| (key.clone(), value)))
                    .collect(),
            )),
            Self::Array(values) => Some(serde_json::Value::Array(
                values
                    .borrow()
                    .iter()
                    .map(|value| value.json_value().unwrap_or(serde_json::Value::Null))
                    .collect(),
            )),
        }
    }

    pub fn stringify(&self) -> Result<Self, Self> {
        match self.json_value() {
            None => Ok(Self::Undefined),
            Some(value) => serde_json::to_string(&value)
                .map(Self::Text)
                .map_err(|error| Self::text(&error.to_string())),
        }
    }
}

#[test]
fn primitive_objects_preserve_identity_and_json_omits_undefined_properties() {
    // Given equal-looking objects with different identities and undefined data.
    let first = Value::object(&[("missing", Value::Undefined), ("flag", Value::Bool(true))]);
    let second = Value::object(&[("flag", Value::Bool(true))]);
    // When raw identity and JSON primitives observe them.
    let cloned = first.clone();
    let json = first.stringify();
    // Then cloning retains identity without structural equality and JSON follows absence rules.
    assert_eq!(first, cloned);
    assert_ne!(first, second);
    assert_eq!(json, Ok(Value::text("{\"flag\":true}")));
    assert_ne!(Value::Null, Value::Undefined);
}
