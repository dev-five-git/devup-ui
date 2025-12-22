use crate::{
    COLOR_HASH, F_SPACE_RE, ZERO_RE,
    constant::{
        DOT_ZERO_RE, F_DOT_RE, F_RGB_RE, F_RGBA_RE, INNER_TRIM_RE, NUM_TRIM_RE, RM_MINUS_ZERO_RE,
        ZERO_PERCENT_FUNCTION,
    },
};

pub fn optimize_value(value: &str) -> String {
    let mut ret = value.trim().to_string();
    ret = INNER_TRIM_RE.replace_all(&ret, "(${1})").to_string();
    ret = RM_MINUS_ZERO_RE.replace_all(&ret, "0${1}").to_string();
    ret = NUM_TRIM_RE.replace_all(&ret, "${1} ${3}").to_string();

    if ret.contains(",") {
        ret = F_SPACE_RE.replace_all(&ret, ",").trim().to_string();
    }
    ret = F_RGBA_RE
        .replace_all(&ret, |c: &regex::Captures| {
            let r = c[1].parse::<i32>().unwrap();
            let g = c[2].parse::<i32>().unwrap();
            let b = c[3].parse::<i32>().unwrap();
            let a = c[4].parse::<f32>().unwrap();
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                r,
                g,
                b,
                (a * 255.0).round() as i32
            )
        })
        .to_string();
    ret = F_RGB_RE
        .replace_all(&ret, |c: &regex::Captures| {
            let r = c[1].parse::<i32>().unwrap();
            let g = c[2].parse::<i32>().unwrap();
            let b = c[3].parse::<i32>().unwrap();
            format!("#{r:02X}{g:02X}{b:02X}")
        })
        .to_string();
    if ret.contains("#") {
        ret = COLOR_HASH
            .replace_all(&ret, |c: &regex::Captures| optimize_color(&c[1]))
            .to_string();
    }
    if ret.contains("0") {
        ret = DOT_ZERO_RE.replace_all(&ret, "${1}0${2}").to_string();
        ret = F_DOT_RE.replace_all(&ret, "${1}.${2}").to_string();
        ret = ZERO_RE.replace_all(&ret, "${1}0").to_string();

        for f in ZERO_PERCENT_FUNCTION.iter() {
            let tmp = ret.to_lowercase();
            if tmp.contains(f) {
                let index = tmp.find(f).unwrap() + f.len();
                let mut zero_idx = vec![];
                let mut depth = 0;
                let chars: Vec<char> = tmp.chars().collect();
                let byte_indices: Vec<usize> = tmp.char_indices().map(|(i, _)| i).collect();

                for (char_idx, &ch) in chars.iter().enumerate().skip(index) {
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth -= 1;
                    } else if ch == '0'
                        && (char_idx == 0 || !chars[char_idx - 1].is_ascii_digit())
                        && (char_idx + 1 >= chars.len() || !chars[char_idx + 1].is_ascii_digit())
                        && depth == 0
                    {
                        zero_idx.push(byte_indices[char_idx]);
                    }
                }
                for i in zero_idx.iter().rev() {
                    ret = ret[..*i].to_string() + "0%" + &ret[*i + 1..];
                }
            }
        }
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

    if ret.contains("(") || ret.contains(")") {
        let mut depth = 0;
        for ch in ret.chars() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
            }
        }
        if depth < 0 {
            for _ in 0..(-depth) {
                ret.insert(0, '(');
            }
        }
        if depth > 0 {
            for _ in 0..depth {
                ret.push(')');
            }
        }
    }
    ret
}

fn optimize_color(value: &str) -> String {
    let mut ret = value.to_uppercase();

    if ret.len() == 6 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] {
            ret = format!("{}{}{}", ch[0], ch[2], ch[4]);
        }
    } else if ret.len() == 8 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] && ch[6] == ch[7] {
            ret = format!("{}{}{}{}", ch[0], ch[2], ch[4], ch[6]);
            if ret.ends_with("F") {
                ret = ret[..ret.len() - 1].to_string();
            }
        } else if ret.ends_with("FF") {
            ret = ret[..ret.len() - 2].to_string();
        }
    } else if ret.len() == 4 && ret.ends_with("F") {
        ret = ret[..ret.len() - 1].to_string();
    }

    format!("#{ret}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("0px", "0")]
    #[case("0.12px", ".12px")]
    #[case("-0.12px", "-.12px")]
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
    #[case("-0px -0px", "0 0")]
    #[case("0.0px   -0px", "0 0")]
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
    #[case("rgba(255,12,12,0.5)", "#FF0C0C80")]
    #[case("rgba(255,12,12,.5)", "#FF0C0C80")]
    #[case("rgba(255,12,12,1)", "#FF0C0C")]
    #[case("rgba(255, 0, 0,    0.5)", "#FF000080")]
    #[case("rgba(255, 255, 255,   0.8  )", "#FFFC")]
    #[case("rgb(255,12,12)", "#FF0C0C")]
    #[case("rgb(255, 0, 0)", "#F00")]
    #[case("rgb(255, 255, 255)", "#FFF")]
    #[case("red;", "red")]
    #[case("translate(0px)", "translate(0)")]
    #[case("translate(-0px,0px)", "translate(0,0)")]
    #[case("translate(-0px, 0px)", "translate(0,0)")]
    #[case("translate(0px, 0px)", "translate(0,0)")]
    #[case("translate(10px, 0px)", "translate(10px,0)")]
    #[case("translate(     10px  , 0px   )", "translate(10px,0)")]
    #[case("translate(     0px  , 0px   )", "translate(0,0)")]
    #[case("         translate(     0px  , 0px   )         ", "translate(0,0)")]
    #[case("clamp(0, 10px, 10px)", "clamp(0%,10px,10px)")]
    #[case("clamp(10px, 0, 10px)", "clamp(10px,0%,10px)")]
    #[case("clamp(10px, 10px, 0)", "clamp(10px,10px,0%)")]
    #[case("clamp(0px, 10px, 0px)", "clamp(0%,10px,0%)")]
    #[case("min(0, 10px)", "min(0%,10px)")]
    #[case("max(0, 10px)", "max(0%,10px)")]
    #[case("min(10px, 0)", "min(10px,0%)")]
    #[case("max(10px, 0)", "max(10px,0%)")]
    #[case("max(some(0), 0)", "max(some(0),0%)")]
    #[case("max(some(0), -0)", "max(some(0),0%)")]
    #[case("translate(0, min(0, 10px))", "translate(0,min(0%,10px))")]
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
    // recovery case
    #[case("max(10px, 0", "max(10px,0%)")]
    #[case("max(10px, calc(0", "max(10px,calc(0%))")]
    #[case("max(10px, any(0", "max(10px,any(0))")]
    #[case("10px, any(0))", "(10px,any(0))")]
    #[case("scale(0deg, 0deg)", "scale(0,0)")]
    #[case(
        "scaleX(0deg) scaleY(0deg) scaleZ(0deg)",
        "scaleX(0) scaleY(0) scaleZ(0)"
    )]
    #[case("scaleX(0deg)", "scaleX(0)")]
    #[case("scaleY(0deg)", "scaleY(0)")]
    #[case("scaleZ(0deg)", "scaleZ(0)")]
    #[case("translate(0px) scale(0deg)", "translate(0) scale(0)")]
    #[case("translate(-0px) scale(-0deg)", "translate(0) scale(0)")]
    #[case("translate(-10px) scale(-10deg)", "translate(-10px) scale(-10deg)")]
    fn test_optimize_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }

    #[rstest]
    #[case("#ff0000", "#F00")]
    #[case("#123456", "#123456")]
    #[case("#ff0000ff", "#F00")]
    #[case("#f00", "#F00")]
    #[case("#f00f", "#F00")]
    #[case("red", "red")]
    #[case("blue", "blue")]
    #[case("transparent", "transparent")]
    fn test_optimize_color(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }
}
