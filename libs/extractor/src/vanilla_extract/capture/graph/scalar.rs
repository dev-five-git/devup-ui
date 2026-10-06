use super::super::super::{js_str, string_code};
use boa_engine::JsValue;
use rustc_hash::FxHashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::vanilla_extract::capture) enum Scalar {
    Text(String),
    Number(String),
    Boolean(bool),
    Null,
    Undefined,
}

impl Scalar {
    pub fn read(value: &JsValue) -> Option<Self> {
        if let Some(text) = js_str(value) {
            Some(Self::Text(text))
        } else if let Some(number) = value.as_number() {
            let code = if number.is_nan() {
                "(0/0)".into()
            } else if number.is_infinite() {
                if number.is_sign_negative() {
                    "(-1/0)"
                } else {
                    "(1/0)"
                }
                .into()
            } else if number.classify() == std::num::FpCategory::Zero && number.is_sign_negative() {
                "(-0)".into()
            } else {
                number.to_string()
            };
            Some(Self::Number(code))
        } else if let Some(value) = value.as_boolean() {
            Some(Self::Boolean(value))
        } else if value.is_null() {
            Some(Self::Null)
        } else if value.is_undefined() {
            Some(Self::Undefined)
        } else {
            None
        }
    }

    pub fn code(&self, names: &FxHashMap<String, String>) -> String {
        match self {
            Self::Text(text) => string_code(text, names),
            Self::Number(code) => code.clone(),
            Self::Boolean(value) => value.to_string(),
            Self::Null => "null".into(),
            Self::Undefined => "undefined".into(),
        }
    }
}
