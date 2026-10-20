use std::collections::BTreeSet;

fn identifier(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

// These fixtures contain unescaped literals and simple template interpolations only.
fn literal_parts(source: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(['"', '\'', '`']) {
        let quote = rest.as_bytes()[start];
        rest = &rest[start + 1..];
        let end = rest
            .bytes()
            .position(|byte| byte == quote)
            .unwrap_or_else(|| panic!("closed fixture literal"));
        let literal = &rest[..end];
        assert!(
            !literal.contains('\\'),
            "unsupported escaped fixture literal: {literal}"
        );
        if quote == b'`' {
            let mut raw = literal;
            while let Some(start) = raw.find("${") {
                parts.push(&raw[..start]);
                let end = raw[start..]
                    .find('}')
                    .unwrap_or_else(|| panic!("closed fixture interpolation"))
                    + start;
                parts.extend(literal_parts(&raw[start + 2..end]));
                raw = &raw[end + 1..];
            }
            parts.push(raw);
        } else {
            parts.push(literal);
        }
        rest = &rest[end + 1..];
    }
    parts
}

pub(super) fn assert_references_resolve(code: &str, css: &str) {
    let mut classes = BTreeSet::new();
    for field in code.split("className=").skip(1) {
        let end = field
            .find(" style=")
            .or_else(|| field.find(" />"))
            .unwrap_or_else(|| panic!("bounded fixture class attribute"));
        for literal in literal_parts(&field[..end]) {
            classes.extend(literal.split_whitespace());
        }
    }
    for field in code.split("className: ").skip(1) {
        let end = field
            .find('}')
            .unwrap_or_else(|| panic!("bounded fixture class object"));
        let literals = literal_parts(&field[..end]);
        assert_eq!(literals.len(), 1);
        classes.extend(literals[0].split_whitespace());
    }
    for value in code.split("const s = { \"root\": ").skip(1) {
        let end = value
            .find('}')
            .unwrap_or_else(|| panic!("bounded fixture StyleX binding"));
        let literals = literal_parts(&value[..end]);
        assert_eq!(literals.len(), 1);
        classes.extend(literals[0].split_whitespace());
    }
    for binding in ["const cls = ", "const card = "] {
        for value in code.split(binding).skip(1) {
            let end = value
                .find(';')
                .unwrap_or_else(|| panic!("bounded fixture class binding"));
            let literals = literal_parts(&value[..end]);
            assert_eq!(literals.len(), 1, "unsupported fixture binding: {value}");
            classes.extend(literals[0].split_whitespace());
        }
    }
    assert_ne!(
        classes.len(),
        0,
        "fixture must generate actual class references: {code}"
    );
    if code.contains("const cls = ") {
        assert_eq!(
            classes.len(),
            if code.contains("const Card = ") {
                10
            } else {
                8
            }
        );
    }
    if code.contains("const card = ") {
        assert_eq!(classes.len(), 3);
    }
    if code.contains("const s = ") {
        assert_eq!(classes.len(), 2);
    }
    let defined: BTreeSet<_> = css
        .split('.')
        .skip(1)
        .map(|tail| tail.split(|c| !identifier(c)).next().unwrap_or_default())
        .collect();
    for name in classes {
        assert!(
            defined.contains(name),
            "unemitted exact class {name}\n{code}\n{css}"
        );
    }
    let mut frames = 0;
    for binding in ["const fade = ", "const a = ", "const b = ", "const c = "] {
        for value in code.split(binding).skip(1) {
            let end = value
                .find(';')
                .unwrap_or_else(|| panic!("bounded fixture animation binding"));
            let literals = literal_parts(&value[..end]);
            assert_eq!(literals.len(), 1);
            let name = literals[0];
            assert!(
                css.contains(&format!("@keyframes {name}{{")),
                "unemitted keyframes {name}"
            );
            assert!(
                css.contains(&format!("animation:{name} ")),
                "unused animation {name}"
            );
            frames += 1;
        }
    }
    let mut variables = 0;
    for tail in code.split("\"--").skip(1) {
        let (name, suffix) = tail
            .split_once('"')
            .unwrap_or_else(|| panic!("closed fixture custom property"));
        assert!(
            suffix.starts_with(':'),
            "fixture variable must be an object key"
        );
        assert!(
            css.contains(&format!("var(--{name})")),
            "unemitted variable --{name}"
        );
        variables += 1;
    }
    if code.contains("const cls = ") {
        assert_eq!(frames, 1);
        assert_eq!(
            variables,
            if code.contains("const Card = ") { 2 } else { 1 }
        );
    }
    if code.contains("const card = ") {
        assert_eq!(frames, 1);
    }
}

#[test]
fn exact_class_validation_rejects_a_local_prefix_match() {
    // Given
    let code = "export const x=<div className=\"a\" />;";
    // When
    let result = std::panic::catch_unwind(|| {
        assert_references_resolve(code, concat!(".a-a{", "color:red}"));
    });
    // Then
    assert!(result.is_err());
}
