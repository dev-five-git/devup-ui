use super::constant::{MAINTAIN_VALUE_PROPERTIES, TIME_PROPERTIES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) enum NumericUnit {
    Length,
    Time,
}

impl NumericUnit {
    pub(crate) fn for_property(property: &str) -> Option<Self> {
        if MAINTAIN_VALUE_PROPERTIES.contains(property) || property.starts_with("--") {
            None
        } else if TIME_PROPERTIES.contains(property) {
            Some(Self::Time)
        } else {
            Some(Self::Length)
        }
    }

    pub(crate) const fn scale(self) -> u8 {
        match self {
            Self::Length => 4,
            Self::Time => 1,
        }
    }

    pub(crate) const fn suffix(self) -> &'static str {
        match self {
            Self::Length => "px",
            Self::Time => "ms",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) enum NumericConversion {
    #[default]
    Keep,
    Number(NumericUnit),
    String(NumericUnit),
    Unknown(NumericUnit),
}

impl NumericConversion {
    pub(crate) fn for_expression(
        property: &str,
        expression: &oxc_ast::ast::Expression<'_>,
    ) -> Self {
        let Some(unit) = NumericUnit::for_property(property) else {
            return Self::Keep;
        };
        if nonnumeric_string(expression) {
            return Self::Keep;
        }
        match crate::source_value_type::classify(expression) {
            crate::source_value_type::ValueType::Number => Self::Number(unit),
            crate::source_value_type::ValueType::String => Self::String(unit),
            crate::source_value_type::ValueType::NonNumericString => Self::Keep,
            crate::source_value_type::ValueType::Unproven => Self::Unknown(unit),
        }
    }

    pub(crate) fn declaration(self, variable: &str) -> String {
        match self {
            Self::Number(unit) => {
                format!("calc(var({variable}) * {}{})", unit.scale(), unit.suffix())
            }
            Self::Keep | Self::String(_) | Self::Unknown(_) => format!("var({variable})"),
        }
    }

    pub(crate) fn inline(self, identifier: &str) -> String {
        let (unit, number) = match self {
            Self::Keep | Self::Number(_) => return identifier.to_string(),
            Self::String(unit) => (unit, false),
            Self::Unknown(unit) => (unit, true),
        };
        let value = "__devupNumber";
        let test = format!(
            r#"typeof {value}==="string"&&/^(?:{})$(?![\s\S])/i.test({value})"#,
            css::numeric_value::PATTERN
        );
        let convert = format!("+{value}*{}+\"{}\"", unit.scale(), unit.suffix());
        let string = format!("{test}?{convert}:{value}");
        let body = if number {
            format!("typeof {value}===\"number\"?{convert}:{string}")
        } else {
            string
        };
        format!("(({value})=>{body})({identifier})")
    }
}

fn nonnumeric_string(expression: &oxc_ast::ast::Expression<'_>) -> bool {
    use oxc_ast::ast::Expression;
    match crate::utils::unwrap_syntax_only(expression) {
        Expression::StringLiteral(value) => css::numeric_value::parse(&value.value).is_none(),
        Expression::TemplateLiteral(value) => value.quasis.last().is_some_and(|quasi| {
            [
                "px", "em", "rem", "vh", "vw", "vmin", "vmax", "%", "ms", "s", "ch", "ex", "cm",
                "mm", "in", "pt", "pc", "dvh", "dvw", "svh", "svw", "lvh", "lvw",
            ]
            .iter()
            .any(|unit| quasi.value.raw.ends_with(unit))
        }),
        Expression::SequenceExpression(value) => {
            value.expressions.last().is_some_and(nonnumeric_string)
        }
        Expression::ConditionalExpression(value) => {
            nonnumeric_string(&value.consequent) && nonnumeric_string(&value.alternate)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{NumericConversion, NumericUnit};
    use crate::{
        assignment_test_support::evaluate, extract_style::extract_static_style::ExtractStaticStyle,
    };

    #[rstest::rstest]
    #[case("flag ? 'auto' : '10px'", NumericConversion::Keep)]
    #[case("flag ? 'auto' : '4'", NumericConversion::Unknown(NumericUnit::Length))]
    #[case("flag ? '4' : 'auto'", NumericConversion::Unknown(NumericUnit::Length))]
    fn conditional_passthrough_requires_both_branches_to_exclude_numeric_strings(
        #[case] source: &str,
        #[case] expected: NumericConversion,
    ) {
        // Given: no source-type scope is active for the fallback syntax classifier.
        let allocator = oxc_allocator::Allocator::default();
        let source = crate::coverage_tests::expression(&allocator, source);
        // When: conversion is chosen for the whole conditional.
        let actual = NumericConversion::for_expression("padding", &source);
        // Then: one numeric-string arm prevents unconditional passthrough.
        assert_eq!(actual, expected);
    }

    #[test]
    fn emitted_string_conversion_agrees_with_static_when_float_spellings_vary() {
        let values = [
            "4", "-2", "0.5", " 4 ", "", ".5", "1.", "1.e2", "1e-2", "+4", "-0", "0x10", "1_000",
            ".", "1e", "4\n", "4\r\n", "auto", "10px", "٤", "1e+2",
        ];
        for value in values {
            let source = serde_json::to_string(value).unwrap_or_else(|error| panic!("{error}"));
            let expression = NumericConversion::String(NumericUnit::Length).inline(&source);
            let actual = evaluate(&format!("JSON.stringify({expression});"));
            let actual =
                serde_json::from_str::<String>(&actual).unwrap_or_else(|error| panic!("{error}"));
            let expected = ExtractStaticStyle::new("padding", value, 0, None);
            assert_eq!(
                css::optimize_value::optimize_value(&actual),
                expected.value,
                "{value:?}"
            );
            let matches = evaluate(&format!(
                r"JSON.stringify(/^(?:{})$(?![\s\S])/i.test({source}));",
                css::numeric_value::PATTERN
            ));
            assert_eq!(
                matches,
                css::numeric_value::parse(value).is_some().to_string(),
                "{value:?}"
            );
        }
    }

    #[test]
    fn unknown_conversion_reads_input_once_when_number_and_string_values_vary() {
        for (input, expected) in [
            ("4", "\"16px\""),
            ("'4'", "\"16px\""),
            ("'auto'", "\"auto\""),
            ("false", "false"),
            ("null", "null"),
        ] {
            let expression = NumericConversion::Unknown(NumericUnit::Length).inline("read()");
            let actual = evaluate(&format!(
                "let reads=0;function read(){{reads++;return {input}}}const value={expression};JSON.stringify([reads,value]);"
            ));
            assert_eq!(actual, format!("[1,{expected}]"));
        }
    }

    #[test]
    fn number_conversion_keeps_raw_value_when_css_applies_the_static_scale() {
        for (unit, declaration) in [
            (NumericUnit::Length, "calc(var(--x) * 4px)"),
            (NumericUnit::Time, "calc(var(--x) * 1ms)"),
        ] {
            let conversion = NumericConversion::Number(unit);
            assert_eq!(conversion.declaration("--x"), declaration);
            assert_eq!(conversion.inline("read()"), "read()");
        }
        assert_eq!(NumericConversion::Keep.declaration("--x"), "var(--x)");
        assert_eq!(NumericConversion::Keep.inline("read()"), "read()");
    }

    #[test]
    fn property_mapping_agrees_with_static_when_lengths_times_and_unitless_props_vary() {
        for property in ["padding", "margin", "width", "border-radius", "gap"] {
            assert_eq!(
                NumericUnit::for_property(property),
                Some(NumericUnit::Length)
            );
        }
        for property in &super::TIME_PROPERTIES {
            assert_eq!(NumericUnit::for_property(property), Some(NumericUnit::Time));
        }
        for property in super::MAINTAIN_VALUE_PROPERTIES
            .iter()
            .chain(std::iter::once(&"--custom"))
        {
            assert_eq!(NumericUnit::for_property(property), None);
        }
        let expression = NumericConversion::String(NumericUnit::Time).inline("'0.5'");
        assert_eq!(
            evaluate(&format!("JSON.stringify({expression});")),
            "\"0.5ms\""
        );
    }
}

#[cfg(test)]
#[path = "../dynamic_scaling_tests.rs"]
mod dynamic_scaling_tests;
