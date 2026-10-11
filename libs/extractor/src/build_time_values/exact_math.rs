use boa_engine::{Context, Source};

/// Math members whose results the local safety gate permits across engines.
pub(crate) const EXACT_MATH: [&str; 20] = [
    "abs", "ceil", "floor", "round", "trunc", "sign", "max", "min", "sqrt", "fround", "imul",
    "clz32", "PI", "E", "LN2", "LN10", "LOG2E", "LOG10E", "SQRT2", "SQRT1_2",
];

pub(crate) enum Operand<'a> {
    Number(f64),
    String(&'a str),
    Bool(bool),
    Null,
    Undefined,
}

impl Operand<'_> {
    fn literal(&self) -> Option<String> {
        match self {
            Self::Number(number) if *number == 0.0 && number.is_sign_negative() => {
                Some("-0".to_string())
            }
            Self::Number(number) => Some(crate::utils::js_number_string(*number)),
            Self::String(text) => serde_json::to_string(text).ok(),
            Self::Bool(value) => Some(value.to_string()),
            Self::Null => Some("null".to_string()),
            Self::Undefined => Some("undefined".to_string()),
        }
    }
}

/// Evaluate only a trusted exact member over already resolved primitive operands.
/// No source expressions, user functions or module code reach Boa here.
pub(crate) fn evaluate(name: &str, arguments: Option<&[Operand<'_>]>) -> Option<f64> {
    if !EXACT_MATH.contains(&name) {
        return None;
    }
    let expression = match arguments {
        None => format!("Math.{name}"),
        Some(arguments) => {
            let arguments: Option<Vec<String>> = arguments.iter().map(Operand::literal).collect();
            format!("Math.{name}({})", arguments?.join(","))
        }
    };
    let value = Context::default()
        .eval(Source::from_bytes(expression.as_bytes()))
        .ok()?
        .as_number()?;
    value.is_finite().then_some(value)
}
