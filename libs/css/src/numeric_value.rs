use regex_lite::Regex;
use std::sync::LazyLock;

/// Shared float grammar for static numeric values and emitted string conversions.
pub const PATTERN: &str =
    r"[+-]?(?:(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|inf(?:inity)?|nan)";

static NUMERIC: LazyLock<Regex> =
    LazyLock::new(|| crate::utils::compile_regex(&format!(r"(?i:\A(?:{PATTERN})\z)")));

/// Read exactly the numeric strings accepted by the shared float grammar.
pub fn parse(value: &str) -> Option<f64> {
    NUMERIC
        .is_match(value)
        .then(|| value.parse().ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn shared_grammar_matches_rust_float_parsing_when_strings_vary() {
        let values = [
            "4",
            "-2",
            "0.5",
            " 4 ",
            "",
            ".5",
            "1.",
            "1.e2",
            "1e-2",
            "+4",
            "-0",
            "1e999",
            "NaN",
            "+NaN",
            "-nan",
            "INF",
            "-infinity",
            "0x10",
            "1_000",
            ".",
            "1e",
            "4\n",
            "auto",
            "10px",
            "٤",
            "1e+2",
            "Infinity",
            "INfiNItY",
            " nan",
        ];
        for value in values {
            let actual = parse(value);
            let expected = value.parse::<f64>().ok();
            assert_eq!(
                actual.map(f64::to_bits),
                expected.map(f64::to_bits),
                "{value:?}"
            );
        }
    }
}
