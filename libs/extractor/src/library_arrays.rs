//! What Emotion, styled-components and vanilla-extract write for a property
//! whose value is an array of literals, where Devup UI reads an array as
//! breakpoints. Each follows its own runtime:
//!
//! - Emotion declares each value in order when the first is a string, skipping
//!   `null`, `undefined` and booleans and making numbers `px`. An array starting
//!   with anything else becomes a nested rule no browser applies.
//! - styled-components declares the property once with the array as a string:
//!   its values joined by commas, `null` and `undefined` written empty.
//! - vanilla-extract declares each value in order as written, numbers without a
//!   unit and `null` as `null`.

use crate::utils::{is_unitless_key, js_number_literal, keeps_bare_number};
use oxc_ast::ast::{ArrayExpression, ArrayExpressionElement, Expression};

/// The library whose rules hold an array
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Library {
    #[default]
    Emotion,
    StyledComponents,
    VanillaExtract,
}

impl Library {
    /// The library an aliased package belongs to
    pub(crate) fn of(source: &str) -> Self {
        match source {
            "styled-components" => Self::StyledComponents,
            "@vanilla-extract/css" => Self::VanillaExtract,
            _ => Self::Emotion,
        }
    }
}

/// What the library declares for an array
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ArrayValue {
    /// The value of the property: its first value, then `;property:value`
    /// for each later one
    Declarations(String),
    /// Nothing a browser applies
    Nothing,
}

/// A literal element of an array
#[derive(Debug, PartialEq)]
enum Literal {
    Text(String),
    Number(f64),
    Null,
    Undefined,
    Bool(bool),
}

impl Literal {
    fn of(element: &ArrayExpressionElement) -> Option<Self> {
        // A hole reads as `undefined`
        if matches!(element, ArrayExpressionElement::Elision(_)) {
            return Some(Self::Undefined);
        }
        let expression = element.as_expression()?;
        if let Some(number) = js_number_literal(expression) {
            return Some(Self::Number(number));
        }
        match expression {
            Expression::StringLiteral(literal) => literal.value.as_str().map(Self::text),
            Expression::TemplateLiteral(template) => template
                .single_quasi()
                .and_then(|quasi| quasi.as_str().map(Self::text)),
            Expression::NullLiteral(_) => Some(Self::Null),
            Expression::BooleanLiteral(literal) => Some(Self::Bool(literal.value)),
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Some(Self::Undefined)
            }
            _ => None,
        }
    }

    fn text(text: &str) -> Self {
        Self::Text(text.to_string())
    }

    /// The value as JavaScript's `String()` writes it
    fn written(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Number(number) => number.to_string(),
            Self::Null => "null".to_string(),
            Self::Undefined => "undefined".to_string(),
            Self::Bool(value) => value.to_string(),
        }
    }
}

/// What `library` declares for `key: array`, if every element is a literal and
/// `key` is a CSS property. A Devup UI shorthand (`p`, `bg`, ...) exists in no
/// such library, so its array keeps Devup UI's breakpoints.
pub(crate) fn library_array(
    library: Library,
    key: &str,
    array: &ArrayExpression,
) -> Option<ArrayValue> {
    if is_devup_shorthand(key) {
        return None;
    }
    let values: Vec<Literal> = array
        .elements
        .iter()
        .map(Literal::of)
        .collect::<Option<_>>()?;
    let declared = match library {
        Library::Emotion => emotion(key, &values),
        Library::StyledComponents => styled_components(&values),
        Library::VanillaExtract => vanilla_extract(key, &values),
    };
    Some(applied(key, declared))
}

/// What a browser applies of `declared` on `key`. A bare number other than
/// zero on a length (`margin: 4`) is invalid there, and Devup UI would read it
/// as its spacing scale, so it declares nothing.
fn applied(key: &str, declared: Option<String>) -> ArrayValue {
    match declared {
        Some(value)
            if is_unitless_key(key)
                || value.parse::<f64>().ok().is_none_or(|number| number == 0.0) =>
        {
            ArrayValue::Declarations(value)
        }
        _ => ArrayValue::Nothing,
    }
}

/// Whether `key` is a Devup UI shorthand, which no other library defines
fn is_devup_shorthand(key: &str) -> bool {
    key != "styleOrder" && !is_unitless_key(key) && keeps_bare_number(key)
}

/// Each value declared in order, skipping empty ones, which a browser drops
fn declare_each(key: &str, values: impl Iterator<Item = String>) -> Option<String> {
    let values: Vec<String> = values.filter(|value| !value.is_empty()).collect();
    (!values.is_empty()).then(|| values.join(&format!(";{}:", css_property(key))))
}

/// Emotion's fallbacks, as `handleInterpolation` in `@emotion/serialize` writes them
fn emotion(key: &str, values: &[Literal]) -> Option<String> {
    if !matches!(values.first(), Some(Literal::Text(_))) {
        return None;
    }
    declare_each(
        key,
        values.iter().filter_map(|value| match value {
            Literal::Text(text) => Some(text.clone()),
            Literal::Number(number) => Some(emotion_number(key, *number)),
            Literal::Null | Literal::Undefined | Literal::Bool(_) => None,
        }),
    )
}

/// A number as Emotion writes it on `key`: `px` unless zero or unitless
fn emotion_number(key: &str, number: f64) -> String {
    if number == 0.0 || is_unitless_key(key) {
        number.to_string()
    } else {
        format!("{number}px")
    }
}

/// styled-components' one declaration of the array as a string, as
/// `objToCssArray` writes it
fn styled_components(values: &[Literal]) -> Option<String> {
    let joined = values
        .iter()
        .map(|value| match value {
            Literal::Null | Literal::Undefined => String::new(),
            value => value.written(),
        })
        .collect::<Vec<_>>()
        .join(",");
    let joined = joined.trim();
    (!joined.is_empty()).then(|| joined.to_string())
}

/// vanilla-extract's fallbacks, each value as written, as `transformCss` writes
/// them
fn vanilla_extract(key: &str, values: &[Literal]) -> Option<String> {
    declare_each(key, values.iter().map(Literal::written))
}

/// `backgroundColor` as the CSS property `background-color`, `WebkitBoxShadow`
/// as `-webkit-box-shadow` and `msFlex` as `-ms-flex`, as each library
/// hyphenates a property. Custom properties stay as written.
pub(crate) fn css_property(key: &str) -> String {
    if key.starts_with("--") {
        return key.to_string();
    }
    let mut property = String::with_capacity(key.len() + 4);
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            property.push('-');
            property.push(c.to_ascii_lowercase());
        } else {
            property.push(c);
        }
    }
    if property.starts_with("ms-") {
        property.insert(0, '-');
    }
    property
}

/// What vanilla-extract declares for `key` holding `values`, each written as
/// JavaScript's `String()` writes it, when `key` is a CSS property: the
/// stylesheet evaluator reads arrays as they run
pub(crate) fn vanilla_extract_values(
    key: &str,
    values: impl Iterator<Item = String>,
) -> Option<ArrayValue> {
    // An element of an outer array, or the whole value, is no property
    if key.is_empty() || key.bytes().all(|byte| byte.is_ascii_digit()) || is_devup_shorthand(key) {
        return None;
    }
    Some(applied(key, declare_each(key, values)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    /// What `library` declares for `key: array`, `array` written as source
    fn declared(library: Library, key: &str, array: &str) -> Option<ArrayValue> {
        let allocator = Allocator::default();
        let source = format!("({array})");
        let program = Parser::new(&allocator, &source, SourceType::ts())
            .parse()
            .program;
        let Some(Statement::ExpressionStatement(statement)) = program.body.first() else {
            panic!("no expression in {source}");
        };
        let Expression::ParenthesizedExpression(inner) = &statement.expression else {
            panic!("no parentheses in {source}");
        };
        let Expression::ArrayExpression(array) = &inner.expression else {
            panic!("no array in {source}");
        };
        library_array(library, key, array)
    }

    fn declarations(value: &str) -> ArrayValue {
        ArrayValue::Declarations(value.to_string())
    }

    // What `@emotion/serialize` 1.3 writes for each array
    #[rstest]
    #[case("margin", "['1px', '3px']", Some(declarations("1px;margin:3px")))]
    #[case(
        "height",
        "['100vh', '100dvh']",
        Some(declarations("100vh;height:100dvh"))
    )]
    #[case("color", "['red', null, 'blue']", Some(declarations("red;color:blue")))]
    #[case(
        "color",
        "['red', undefined, false, true, 'blue']",
        Some(declarations("red;color:blue"))
    )]
    #[case("color", "['red', , 'blue']", Some(declarations("red;color:blue")))]
    #[case(
        "margin",
        "['4px', 8, 0, -2]",
        Some(declarations("4px;margin:8px;margin:0;margin:-2px"))
    )]
    #[case("zIndex", "['1', 2]", Some(declarations("1;z-index:2")))]
    #[case("--gap", "['1px', 2]", Some(declarations("1px;--gap:2")))]
    #[case("color", "[`red`, 'blue']", Some(declarations("red;color:blue")))]
    #[case("color", "['', 'red']", Some(declarations("red")))]
    #[case("color", "['']", Some(ArrayValue::Nothing))]
    #[case("margin", "[4]", Some(ArrayValue::Nothing))]
    #[case("margin", "[4, '8px']", Some(ArrayValue::Nothing))]
    #[case("color", "[null, 'red']", Some(ArrayValue::Nothing))]
    #[case("color", "[]", Some(ArrayValue::Nothing))]
    #[case("color", "['red', x]", None)]
    #[case("color", "['red', `${x}`]", None)]
    #[case("color", "['red', ...rest]", None)]
    #[case("p", "['1px', '2px']", None)]
    fn emotion_arrays(
        #[case] key: &str,
        #[case] array: &str,
        #[case] expected: Option<ArrayValue>,
    ) {
        assert_eq!(
            declared(Library::Emotion, key, array),
            expected,
            "{key}: {array}"
        );
    }

    // What styled-components 6 writes for each array: one declaration
    #[rstest]
    #[case("margin", "['1px', '3px']", Some(declarations("1px,3px")))]
    #[case(
        "fontFamily",
        "['Arial', 'sans-serif']",
        Some(declarations("Arial,sans-serif"))
    )]
    #[case("zIndex", "[4]", Some(declarations("4")))]
    #[case("margin", "[0]", Some(declarations("0")))]
    #[case("margin", "[4]", Some(ArrayValue::Nothing))]
    #[case("color", "['red', null, 'blue']", Some(declarations("red,,blue")))]
    #[case("color", "['red', undefined, false]", Some(declarations("red,,false")))]
    #[case("color", "[' red ', true]", Some(declarations("red ,true")))]
    #[case("color", "[null]", Some(ArrayValue::Nothing))]
    #[case("color", "[]", Some(ArrayValue::Nothing))]
    fn styled_components_arrays(
        #[case] key: &str,
        #[case] array: &str,
        #[case] expected: Option<ArrayValue>,
    ) {
        assert_eq!(
            declared(Library::StyledComponents, key, array),
            expected,
            "{key}: {array}"
        );
    }

    // What vanilla-extract's `transformCss` writes for each array
    #[rstest]
    #[case("margin", "['1px', '3px']", Some(declarations("1px;margin:3px")))]
    #[case("margin", "[4]", Some(ArrayValue::Nothing))]
    #[case(
        "color",
        "['red', null, undefined, false]",
        Some(declarations("red;color:null;color:undefined;color:false"))
    )]
    #[case("msFlex", "['1', '2']", Some(declarations("1;-ms-flex:2")))]
    #[case("color", "['']", Some(ArrayValue::Nothing))]
    fn vanilla_extract_arrays(
        #[case] key: &str,
        #[case] array: &str,
        #[case] expected: Option<ArrayValue>,
    ) {
        assert_eq!(
            declared(Library::VanillaExtract, key, array),
            expected,
            "{key}: {array}"
        );
    }

    #[test]
    fn vanilla_extract_values_skip_what_is_no_property() {
        let values = || ["red".to_string(), "blue".to_string()].into_iter();
        assert_eq!(vanilla_extract_values("", values()), None);
        assert_eq!(vanilla_extract_values("0", values()), None);
        assert_eq!(vanilla_extract_values("bg", values()), None);
        assert_eq!(
            vanilla_extract_values("color", values()),
            Some(declarations("red;color:blue"))
        );
        assert_eq!(
            vanilla_extract_values("color", std::iter::empty()),
            Some(ArrayValue::Nothing)
        );
    }

    #[rstest]
    #[case("@emotion/react", Library::Emotion)]
    #[case("@emotion/css", Library::Emotion)]
    #[case("styled-components", Library::StyledComponents)]
    #[case("@vanilla-extract/css", Library::VanillaExtract)]
    fn libraries_of_packages(#[case] source: &str, #[case] library: Library) {
        assert_eq!(Library::of(source), library);
    }

    #[rstest]
    #[case("backgroundColor", "background-color")]
    #[case("WebkitBoxShadow", "-webkit-box-shadow")]
    #[case("MozAppearance", "-moz-appearance")]
    #[case("msFlex", "-ms-flex")]
    #[case("--myColor", "--myColor")]
    fn css_properties(#[case] key: &str, #[case] property: &str) {
        assert_eq!(css_property(key), property);
    }
}
