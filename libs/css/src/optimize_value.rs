use crate::{
    COLOR_HASH, F_SPACE_RE, ZERO_RE,
    constant::{DOT_ZERO_RE, F_DOT_RE},
};

pub fn optimize_value(value: &str) -> String {
    let mut ret = value.trim().to_string();
    if ret.contains(",") {
        ret = F_SPACE_RE.replace_all(&ret, ",").trim().to_string();
    }
    if ret.contains("#") {
        ret = COLOR_HASH
            .replace_all(&ret, |c: &regex::Captures| optimize_color(&c[1]))
            .to_string();
    }
    if ret.contains("0") {
        ret = DOT_ZERO_RE.replace_all(&ret, "${1}0${2}").to_string();
        ret = F_DOT_RE.replace_all(&ret, "${1}.${2}").to_string();
        ret = ZERO_RE.replace_all(&ret, "${1}0").to_string();
    }
    // remove ; from dynamic value
    for str_symbol in ["", "`", "\"", "'"] {
        if ret.ends_with(&format!(";{str_symbol}")) {
            ret = format!(
                "{}{}",
                ret[..ret.len() - str_symbol.len() - 1].trim_end_matches(';'),
                str_symbol
            );
        } else if ret.ends_with(&format!(";{str_symbol})")) {
            ret = format!(
                "{}{})",
                ret[..ret.len() - str_symbol.len() - 2].trim_end_matches(';'),
                str_symbol
            );
        }
    }
    ret
}

fn optimize_color(value: &str) -> String {
    let mut ret = value.to_string().to_uppercase();

    if ret.len() == 6 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] {
            ret = format!("{}{}{}", ch[0], ch[2], ch[4]);
        }
    } else if ret.len() == 8 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] && ch[6] == ch[7] {
            ret = format!("{}{}{}{}", ch[0], ch[2], ch[4], ch[6]);
        }
    }

    format!("#{ret}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("0px", "0")]
    #[case("0.0px", "0")]
    #[case("0.0em", "0")]
    #[case("0.0rem", "0")]
    #[case("0.0vh", "0")]
    #[case("0.0vw", "0")]
    #[case("0.0%", "0")]
    #[case("0.0dvh", "0")]
    #[case("0.0dvw", "0")]
    #[case("1.3s", "1.3s")]
    #[case("0.3s", ".3s")]
    #[case("0.3s ease-in-out", ".3s ease-in-out")]
    #[case("0em", "0")]
    #[case("0rem", "0")]
    #[case("0vh", "0")]
    #[case("0vw", "0")]
    #[case("0%", "0")]
    #[case("0dvh", "0")]
    #[case("0dvw", "0")]
    #[case("0px 0px", "0 0")]
    #[case("0em 0em", "0 0")]
    #[case("0rem 0rem", "0 0")]
    #[case("0vh 0vh", "0 0")]
    #[case("0vw 0vw", "0 0")]
    #[case("-0vw -0vw", "0 0")]
    #[case("-0.2em", "-.2em")]
    #[case("-0.02em", "-.02em")]
    #[case("scale(0px)", "scale(0)")]
    #[case("scale(-0px)", "scale(0)")]
    #[case("scale(-0px);", "scale(0)")]
    #[case("rgba(255, 0, 0,    0.5)", "rgba(255,0,0,.5)")]
    #[case("rgba(0.0,0.0,0.0,0.5)", "rgba(0,0,0,.5)")]
    #[case("red;", "red")]
    #[case("translate(0px)", "translate(0)")]
    #[case("translate(-0px,0px)", "translate(0,0)")]
    #[case("translate(-0px, 0px)", "translate(0,0)")]
    #[case("translate(0px, 0px)", "translate(0,0)")]
    #[case("translate(10px, 0px)", "translate(10px,0)")]
    #[case("\"red\"", "\"red\"")]
    #[case("'red'", "'red'")]
    #[case("`red`", "`red`")]
    #[case("\"red;\"", "\"red\"")]
    #[case("'red;'", "'red'")]
    #[case("`red;`", "`red`")]
    #[case("(\"red;\")", "(\"red\")")]
    #[case("(`red;`)", "(`red`)")]
    #[case("('red;')", "('red')")]
    #[case("('red') + 'blue;'", "('red') + 'blue'")]
    #[case("translateX(0px) translateY(0px)", "translateX(0) translateY(0)")]
    fn test_optimize_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }

    #[rstest]
    #[case("#ff0000", "#F00")]
    #[case("#123456", "#123456")]
    #[case("#ff0000ff", "#F00F")]
    #[case("#f00", "#F00")]
    #[case("#f00f", "#F00F")]
    #[case("red", "red")]
    #[case("blue", "blue")]
    #[case("transparent", "transparent")]
    fn test_optimize_color(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }
}
