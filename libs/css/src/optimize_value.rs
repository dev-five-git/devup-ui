use std::borrow::Cow;

use crate::constant::{
    COLOR_HASH, DOT_ZERO_RE, F_DOT_RE, F_RGB_RE, F_RGBA_RE, F_SPACE_RE, INNER_TRIM_RE, NUM_TRIM_RE,
    RM_MINUS_ZERO_RE, ZERO_PERCENT_FUNCTION, ZERO_RE,
};

/// (symbol, ";{symbol}", ";{symbol})") — compile-time constants, zero probe allocation
const SEMI_SUFFIXES: [(&str, &str, &str); 4] = [
    ("", ";", ";)"),
    ("`", ";`", ";`)"),
    ("\"", ";\"", ";\")"),
    ("'", ";'", ";')"),
];

pub fn optimize_value(value: &str) -> Cow<'_, str> {
    let trimmed = value.trim();

    // ONE pre-scan of `trimmed` for every byte flag the passes below key off.
    // This subsumes the two former folds (the `--`-wrap space/comma probe and the
    // post-wrap `(has_open_paren, saw_close_paren, has_double_dash, _, saw_semi)`
    // fold) AND feeds a borrow fast path: every mutating pass in this function
    // syntactically REQUIRES at least one of these bytes to fire —
    //   - var() wrap: leading `--` (⇒ `has_double_dash`)
    //   - INNER_TRIM_RE `\(\s*([^)]*?)\s*\)`: a `(`
    //   - RM_MINUS_ZERO_RE `-0(px|…|\)|,)`: a `0`
    //   - NUM_TRIM_RE `(\d(unit)?)\s+(\d)`: ASCII whitespace
    //   - F_SPACE_RE `\s*,\s*` (gated on `contains(',')`): a `,`
    //   - F_RGBA_RE / F_RGB_RE (gated on `contains("rgba(")`/`contains("rgb(")`): a `(`
    //   - COLOR_HASH `#([0-9a-zA-Z]+)`: a `#`
    //   - DOT_ZERO_RE / F_DOT_RE / ZERO_RE / ZERO_PERCENT (all inside `has_zero`): a `0`
    //   - SEMI_SUFFIXES strip: a `;`
    //   - unbalanced-paren fixup: a `(` or `)`
    // so a value with NONE of them set is provably returned byte-for-byte
    // unchanged and can be borrowed instead of copied into a fresh `String`.
    // `has_space` (a literal `' '`) is tracked separately from `has_ws` (any
    // ASCII whitespace) because the var()-wrap condition only rejects `' '`/`,`,
    // exactly like the fold it replaces.
    let mut has_space = false;
    let mut has_ws = false;
    let mut has_comma = false;
    let mut pre_open_paren = false;
    let mut pre_close_paren = false;
    let mut has_double_dash = false;
    let mut saw_semi = false;
    let mut has_hash = false;
    let mut has_zero = false;
    let mut prev_dash = false;
    for b in trimmed.bytes() {
        match b {
            b' ' => {
                has_space = true;
                has_ws = true;
            }
            b'\t' | b'\n' | b'\x0C' | b'\r' => has_ws = true,
            b',' => has_comma = true,
            b'(' => pre_open_paren = true,
            b')' => pre_close_paren = true,
            b';' => saw_semi = true,
            b'#' => has_hash = true,
            b'0' => has_zero = true,
            b'-' if prev_dash => has_double_dash = true,
            _ => {}
        }
        prev_dash = b == b'-';
    }

    // FAST PATH: no flag set ⇒ no pass can modify `trimmed` (see table above),
    // so hand the caller a borrow of the trimmed input. This covers the dominant
    // plain values (`red`, `14px`, `$primary`, `1.3s`) with zero allocation.
    // A leading `--` raises `has_double_dash` (two consecutive dashes), so the
    // var()-wrap path can never be skipped by this early return.
    if !has_ws
        && !has_comma
        && !pre_open_paren
        && !pre_close_paren
        && !has_double_dash
        && !saw_semi
        && !has_hash
        && !has_zero
    {
        return Cow::Borrowed(trimmed);
    }

    let mut ret = String::with_capacity(trimmed.len() + 8);

    // Wrap CSS custom property names in var() when used as values
    // e.g., "--var-0" becomes "var(--var-0)". Probe `trimmed` up front so the
    // buffer is built FORWARD (`var(` + trimmed + `)`) instead of pushing
    // `trimmed` then `insert_str(0, "var(")` — the latter memmoves the whole
    // buffer right by 4 bytes on every custom-prop value. Output is byte-identical.
    // `--`-prefixed values are wrapped in `var()` only when they hold no space and no
    // comma; both flags come from the single pre-scan above.
    let wrapped_custom_prop = trimmed.starts_with("--") && !has_space && !has_comma;
    if wrapped_custom_prop {
        ret.push_str("var(");
        ret.push_str(trimmed);
        ret.push(')');
    } else {
        ret.push_str(trimmed);
    }

    // Derive the post-wrap flags the passes below gate on from the pre-scan:
    // the var() wrap only adds `var(` + `)` — one `(`, one `)`, no `;` — so
    //   - `has_open_paren` / `may_have_paren` are seeded `true` by the wrap,
    //   - `saw_semi` is unchanged by the wrap (folded before any regex pass runs;
    //     no later replacement can introduce a `;`: every one either inserts fixed
    //     literals with no `;` (INNER_TRIM `(${1})`, F_SPACE `,`, F_RGBA/F_RGB/
    //     COLOR_HASH hex, F_DOT `${1}.${2}`, ZERO `${1}0`, ZERO_PERCENT `%`) or
    //     re-emits bytes captured FROM `ret` (RM_MINUS_ZERO `0${1}`, NUM_TRIM
    //     `${1} ${3}`, DOT_ZERO `${1}0${2}`)).
    // The paren flags are only ever allowed to *skip* when provably paren-free:
    // seeded `true` on any `(`/`)` present (or the var()-wrap) and never cleared —
    // the rgba/rgb hex replacements only REMOVE parens, so leaving the flag set
    // there is conservative (runs a redundant scan, never misses a needed fix).
    let has_open_paren = wrapped_custom_prop || pre_open_paren;
    let may_have_paren = wrapped_custom_prop || pre_open_paren || pre_close_paren;

    // When the var()-wrap above fired, the value started with `--`, so the flag is
    // trivially true. Otherwise a value can still carry an interior `--` (e.g. a
    // pre-wrapped `var(--x)`), captured by `has_double_dash` above. INNER_TRIM_RE only
    // wraps its capture in parens and cannot add or remove `--`, so the flag stays valid
    // across that step (the fold runs before it, same as the old `contains("--")` did).
    let has_custom_prop = wrapped_custom_prop || has_double_dash;

    // Use Cow-aware replacement: only allocate when regex matches.
    // INNER_TRIM_RE = `\(\s*([^)]*?)\s*\)` requires a `(` to match; the only code
    // that can introduce a `(` before here is the var() wrap above. Probe the
    // post-wrap buffer so we skip the regex-engine setup on the common no-paren
    // values (`red`, `14px`, `$primary`, `0px`) — matching the existing
    // `if ret.contains(',')` / `if ret.contains("rgba(")` guard style below.
    if has_open_paren {
        let replaced = INNER_TRIM_RE.replace_all(&ret, "(${1})");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }

    // Skip RM_MINUS_ZERO_RE for values containing CSS custom property references
    // to preserve names like --var-0 (the -0 should not be converted to 0)
    if !has_custom_prop {
        let replaced = RM_MINUS_ZERO_RE.replace_all(&ret, "0${1}");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    // NUM_TRIM_RE = `(\d(unit)?)\s+(\d)` needs `\s+` to match. `value` was
    // trim()-ed above so only interior whitespace can remain; a value with none
    // (`red`, `14px`, `0px`, `$primary`) can never match — skip the regex pass.
    // `\s` in regex_lite is ASCII-only (`[ \t\n\r\x0b\x0c]`), so an ASCII byte
    // scan is a sound (and cheaper, non-Unicode) gate matching the sibling scans.
    if ret.bytes().any(|b| b.is_ascii_whitespace()) {
        let replaced = NUM_TRIM_RE.replace_all(&ret, "${1} ${3}");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }

    if ret.contains(',') {
        let replaced = F_SPACE_RE.replace_all(&ret, ",");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    if ret.contains("rgba(") {
        let replaced = F_RGBA_RE.replace_all(&ret, |c: &regex_lite::Captures| {
            match (
                c[1].parse::<i32>(),
                c[2].parse::<i32>(),
                c[3].parse::<i32>(),
                c[4].parse::<f32>(),
            ) {
                (Ok(r), Ok(g), Ok(b), Ok(a)) => format!(
                    "#{:02X}{:02X}{:02X}{:02X}",
                    r,
                    g,
                    b,
                    (a * 255.0).round() as i32
                ),
                _ => c[0].to_string(),
            }
        });
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    if ret.contains("rgb(") {
        let replaced = F_RGB_RE.replace_all(&ret, |c: &regex_lite::Captures| {
            match (
                c[1].parse::<i32>(),
                c[2].parse::<i32>(),
                c[3].parse::<i32>(),
            ) {
                (Ok(r), Ok(g), Ok(b)) => format!("#{r:02X}{g:02X}{b:02X}"),
                _ => c[0].to_string(),
            }
        });
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    // Detect `#` and `0` in a SINGLE byte pass, replacing the two back-to-back
    // full-string `contains('#')` / `contains('0')` scans below. Both booleans
    // gate the same branches as before, so output is byte-identical; the fold
    // just avoids a redundant O(n) traversal on every value (e.g. `14px` used to
    // pay two scans to find neither, `#FF0000` two scans that overlap).
    // Also track `.` in the same byte pass: DOT_ZERO_RE (`(\b|,)-?0\.0+([^\d])`)
    // and F_DOT_RE (`(\b|,)0\.(\d+)`) both syntactically REQUIRE a literal `.` to
    // match, so a dot-free value (`0px`, `#FF0000`, `10px 0`, `translate(0px, 0px)`)
    // can never match either — yet the `has_zero` block used to run both full
    // regex `replace_all` passes on every zero-bearing value. Gate them on
    // `has_dot` so the common no-dot zero values skip two NFA executions. Output
    // is byte-identical: neither regex can alter a string with no `.`.
    let (has_hash, has_zero, has_dot) = ret.bytes().fold((false, false, false), |(h, z, d), b| {
        (h || b == b'#', z || b == b'0', d || b == b'.')
    });
    if has_hash {
        let replaced =
            COLOR_HASH.replace_all(&ret, |c: &regex_lite::Captures| optimize_color(&c[1]));
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    if has_zero {
        // DOT_ZERO_RE and F_DOT_RE both require a `.`; keep them first (preserving
        // the original replacement order) but only when a `.` is present.
        if has_dot {
            let replaced = DOT_ZERO_RE.replace_all(&ret, "${1}0${2}");
            if let std::borrow::Cow::Owned(s) = replaced {
                ret = s;
            }
            let replaced = F_DOT_RE.replace_all(&ret, "${1}.${2}");
            if let std::borrow::Cow::Owned(s) = replaced {
                ret = s;
            }
        }
        let replaced = ZERO_RE.replace_all(&ret, "${1}0");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }

        // Every ZERO_PERCENT_FUNCTION token ends in '(', so a value with no '('
        // can never match. Skip the lowercase allocation and scan entirely on the
        // common no-paren path (colors like #FF0000, plain lengths like `10px 0`).
        // When a '(' is present, allocate the lowercase copy once and let the
        // per-token `lower.find(f)` loop below no-op on non-matching tokens —
        // that loop already performs the definitive scan, so a separate
        // case-insensitive pre-scan (formerly `any(contains_ci)`) would only
        // duplicate the work it does.
        if ret.contains('(') {
            // Lowercase ONCE for case-insensitive function-name matching. The
            // previous version re-lowercased `ret` after every modified token
            // (a full-string heap allocation per math function), which is pure
            // churn on multi-function values like `clamp(...) + min(...)`.
            //
            // Instead, collect every zero position to convert across ALL matched
            // functions against this single immutable `lower`, then apply them to
            // `ret` in one back-to-front pass. This is byte-identical to the old
            // per-token refresh: replacements only ever insert a `%` immediately
            // after a top-level `0`, and a convertible `0` is by construction never
            // digit-adjacent (a `0` next to another digit is skipped), so an
            // inserted `%` can never sit beside another convertible `0` nor change
            // any later depth/zero scan. Applying the collected indices highest-first
            // keeps every earlier (lower) index valid despite the +1 byte growth.
            let lower = ret.to_lowercase();
            let bytes = lower.as_bytes();
            let mut zero_idx: Vec<usize> = Vec::with_capacity(2);
            for f in &ZERO_PERCENT_FUNCTION {
                if let Some(start) = lower.find(f) {
                    let index = start + f.len();
                    let mut depth: i32 = 0;
                    for i in index..bytes.len() {
                        match bytes[i] {
                            b'(' => depth += 1,
                            b')' => depth -= 1,
                            b'0' if depth == 0
                                && (i == 0 || !bytes[i - 1].is_ascii_digit())
                                && (i + 1 >= bytes.len() || !bytes[i + 1].is_ascii_digit()) =>
                            {
                                zero_idx.push(i);
                            }
                            _ => {}
                        }
                    }
                }
            }
            if !zero_idx.is_empty() {
                // Apply highest-index-first so earlier indices stay valid as each
                // `0` grows to `0%` (+1 byte). Dedup guards against the same top-level
                // `0` being collected by two overlapping function matches.
                zero_idx.sort_unstable();
                zero_idx.dedup();
                for i in zero_idx.iter().rev() {
                    ret.replace_range((*i)..=(*i), "0%");
                }
            }
        }
    }
    // remove ; from dynamic value. Every SEMI_SUFFIXES entry contains `;`, so a
    // value with no `;` can never match — skip the 4 strip_suffix probes entirely.
    // `saw_semi` was folded from the SAME post-wrap byte scan above (no later pass can
    // add a `;`), so this reuses that result instead of a fresh `contains(';')` scan.
    //
    // Do NOT `break` after a match. `expression_to_code` serializes the value inside
    // an `ExpressionStatement`, so a quoted dynamic value can arrive carrying BOTH an
    // inner `;` before its closing symbol AND the appended statement terminator — e.g.
    // ``expression_to_code(`${color};`)`` yields ``` `${color};`; ```. Entry 0 (`;`)
    // strips the bare terminator, leaving `` `${color};` `` whose inner `;` only the
    // backtick entry can strip. The suffixes are therefore NOT mutually exclusive at
    // the end of such a value, so every entry must get a turn (this mirrors the
    // original pre-`SEMI_SUFFIXES` loop; a premature `break` regressed the
    // `remove_semicolon` case). At most one strip fires per entry, and any strip
    // leaves `ret` ending in `str_symbol`/`)`, so the pass still terminates in ≤4 steps.
    if saw_semi {
        for (str_symbol, suffix_without_paren, suffix_with_paren) in SEMI_SUFFIXES {
            if let Some(stripped) = ret.strip_suffix(suffix_without_paren) {
                let base = stripped.trim_end_matches(';');
                let mut new_ret = String::with_capacity(base.len() + str_symbol.len());
                new_ret.push_str(base);
                new_ret.push_str(str_symbol);
                ret = new_ret;
            } else if let Some(stripped) = ret.strip_suffix(suffix_with_paren) {
                let base = stripped.trim_end_matches(';');
                let mut new_ret = String::with_capacity(base.len() + str_symbol.len() + 1);
                new_ret.push_str(base);
                new_ret.push_str(str_symbol);
                new_ret.push(')');
                ret = new_ret;
            }
        }
    }

    // Single pass to detect unbalanced parens: accumulate depth over the whole
    // string while tracking whether any paren was seen. This folds the former
    // two-probe guard (`ret.contains('(') || ret.contains(')')` — up to two full
    // byte scans) into the SAME loop that already scans every byte, so the common
    // no-paren values (`red`, `14px`, `$primary`, `0px`, `#FF0000`) pay exactly
    // one scan instead of two, and the `saw_paren` fast-out preserves the "no
    // paren ⇒ no mutation" behavior. Byte-identical output.
    //
    // `may_have_paren` gates the scan entirely: it was seeded `true` for any value
    // that ever held a `(` or `)` (and only ever set, never cleared), so a `false`
    // flag PROVES `ret` is paren-free and the scan can only ever no-op. The common
    // paren-free values skip this whole-string traversal.
    if may_have_paren {
        let mut depth: i32 = 0;
        let mut saw_paren = false;
        for b in ret.bytes() {
            if b == b'(' {
                depth += 1;
                saw_paren = true;
            } else if b == b')' {
                depth -= 1;
                saw_paren = true;
            }
        }
        if saw_paren {
            if depth < 0 {
                ret.insert_str(0, &"(".repeat(depth.unsigned_abs() as usize));
            } else if depth > 0 {
                for _ in 0..depth {
                    ret.push(')');
                }
            }
        }
    }
    Cow::Owned(ret)
}

fn optimize_color(value: &str) -> String {
    // `value` is an ASCII hex capture (`COLOR_HASH` = `#([0-9a-fA-F]+)`), so
    // ASCII-only uppercasing is correct and skips the Unicode-aware casing tables.
    // Build the result in one pass into a fresh buffer seeded with '#', so the
    // collapse branches push the final bytes directly and we avoid both the
    // clear/re-push churn and the front `insert(0, '#')` memmove.
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len() + 1);
    out.push('#');

    // Uppercase a single ASCII hex byte (a-f -> A-F, digits/A-F unchanged).
    let up = |b: u8| b.to_ascii_uppercase();

    match bytes.len() {
        6 if bytes[0].eq_ignore_ascii_case(&bytes[1])
            && bytes[2].eq_ignore_ascii_case(&bytes[3])
            && bytes[4].eq_ignore_ascii_case(&bytes[5]) =>
        {
            out.push(up(bytes[0]) as char);
            out.push(up(bytes[2]) as char);
            out.push(up(bytes[4]) as char);
        }
        8 => {
            // Collapse the two former `len()==8` arms into one, evaluating each
            // pair-equality and the trailing-alpha opacity ONCE. Order is preserved:
            // the nibble-duplication collapse (`#RRGGBBAA→#RGB(A)`) is still tried
            // BEFORE the opaque `#RRGGBBFF→#RRGGBB` collapse, so load-bearing cases
            // like `#ff0000ff → #F00` stay byte-identical. Previously a non-nibble
            // opaque color re-ran the opacity check after the four failed
            // `eq_ignore_ascii_case` comparisons; now `alpha_opaque` is computed once.
            let alpha_opaque = up(bytes[6]) == b'F' && up(bytes[7]) == b'F';
            if bytes[0].eq_ignore_ascii_case(&bytes[1])
                && bytes[2].eq_ignore_ascii_case(&bytes[3])
                && bytes[4].eq_ignore_ascii_case(&bytes[5])
                && bytes[6].eq_ignore_ascii_case(&bytes[7])
            {
                out.push(up(bytes[0]) as char);
                out.push(up(bytes[2]) as char);
                out.push(up(bytes[4]) as char);
                // A trailing `F` alpha pair (fully opaque) collapses away entirely.
                // `bytes[6]==bytes[7]` here, so `alpha_opaque` == `up(bytes[6])=='F'`.
                if !alpha_opaque {
                    out.push(up(bytes[6]) as char);
                }
            } else if alpha_opaque {
                for &b in &bytes[..6] {
                    out.push(up(b) as char);
                }
            } else {
                for &b in bytes {
                    out.push(up(b) as char);
                }
            }
        }
        4 if up(bytes[3]) == b'F' => {
            for &b in &bytes[..3] {
                out.push(up(b) as char);
            }
        }
        _ => {
            for &b in bytes {
                out.push(up(b) as char);
            }
        }
    }

    out
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
    // `expression_to_code` serializes the value inside an `ExpressionStatement`,
    // appending a statement `;`. A quoted value that already ends in an inner `;`
    // therefore arrives with TWO trailing semicolons; BOTH must be stripped.
    // Regression guard for `remove_semicolon` (a premature `break` used to keep the
    // inner one, e.g. `` `${color};` ``).
    #[case("`red;`;", "`red`")]
    #[case("\"red;\";", "\"red\"")]
    #[case("'red;';", "'red'")]
    #[case("`${color};`;", "`${color}`")]
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
    // rgba/rgb fallback paths when channel parsing fails
    // i32 overflow for r/g/b (> i32::MAX = 2_147_483_647) → falls back to original
    #[case("rgba(2147483648,0,0,0.5)", "rgba(2147483648,0,0,.5)")]
    #[case("rgb(2147483648,0,0)", "rgb(2147483648,0,0)")]
    // f32 parse failure for alpha (".") → falls back to original
    #[case("rgba(255,0,0,.)", "rgba(255,0,0,.)")]
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

    #[rstest]
    #[case("--var-0", "var(--var-0)")]
    #[case("--my-custom-prop", "var(--my-custom-prop)")]
    #[case("--primary-color", "var(--primary-color)")]
    #[case("var(--var-0)", "var(--var-0)")] // Already wrapped, don't double wrap
    #[case("--a --b", "--a --b")] // Contains space, don't wrap
    #[case("--a, --b", "--a,--b")] // Contains comma, don't wrap (spaces after commas are removed)
    fn test_css_custom_property_wrapping(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }
}
