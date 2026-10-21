use super::*;

#[derive(Clone, Copy, Debug)]
enum StyleShape {
    Explicit,
    Vars,
    Combined,
    ComputedValue,
    ComputedKey,
    ComputedBoth,
}

impl StyleShape {
    fn props(self, binding: Binding) -> String {
        let value = binding.call();
        let key = value.replace("overflow: 'hidden'", "opacity: 0.5");
        match self {
            Self::Explicit => format!("style: {{ '--own-class': {value} }}"),
            Self::Vars => format!("styleVars: {{ ownClass: {value} }}"),
            Self::Combined => {
                format!("styleVars: {{ ownClass: {value} }}, style: {{ '--explicit-key': {key} }}")
            }
            Self::ComputedValue => format!("styleVars: {{ [h ? 'gap-a' : 'gap-b']: {value} }}"),
            Self::ComputedKey => format!("styleVars: {{ [{key}]: 'fixed' }}"),
            Self::ComputedBoth => format!("styleVars: {{ [{key}]: {value} }}"),
        }
    }

    fn computed_key(self) -> bool {
        matches!(self, Self::ComputedKey | Self::ComputedBoth)
    }

    fn key(self, h: bool) -> &'static str {
        match self {
            Self::Explicit => "--own-class",
            Self::Vars | Self::Combined => "--ownClass",
            Self::ComputedValue => {
                if h {
                    "--gap-a"
                } else {
                    "--gap-b"
                }
            }
            Self::ComputedKey | Self::ComputedBoth => "--local-key",
        }
    }

    fn trace(self, binding: Binding) -> Vec<String> {
        if !binding.shadow() {
            return vec![];
        }
        match self {
            Self::Explicit | Self::Vars | Self::ComputedValue => vec!["value:hidden".into()],
            Self::Combined => vec!["value:hidden".into(), "key:0.5".into()],
            Self::ComputedKey => vec!["key:0.5".into()],
            Self::ComputedBoth => vec!["key:0.5".into(), "value:hidden".into()],
        }
    }
}

fn style_argument(binding: Binding) -> &'static str {
    const LOCAL: &str = "rules => { if ('opacity' in rules) { trace.push('key:' + rules.opacity); return 'local-key'; } trace.push('value:' + rules.overflow); return 'local-value'; }";
    match binding {
        Binding::Alias | Binding::Namespace => "void 0",
        Binding::ShadowAlias => LOCAL,
        Binding::ShadowNamespace => {
            "{ css: rules => { if ('opacity' in rules) { trace.push('key:' + rules.opacity); return 'local-key'; } trace.push('value:' + rules.overflow); return 'local-value'; } }"
        }
    }
}

fn style_invocation(shape: StyleShape, binding: Binding, h: bool) -> String {
    let expected_count = if matches!(shape, StyleShape::Combined) {
        2
    } else {
        1
    };
    let key_check = if shape.computed_key() && !binding.shadow() {
        "keys[0].startsWith('--')".to_string()
    } else {
        format!("keys.includes({:?})", shape.key(h))
    };
    let value_check = match (shape, binding.shadow()) {
        (StyleShape::ComputedKey, _) => "style[keys[0]] === 'fixed'",
        (StyleShape::Combined, true) => {
            "style['--ownClass'] === 'local-value' && style['--explicit-key'] === 'local-key'"
        }
        (_, true) => "style[keys[0]] === 'local-value'",
        (_, false) => "typeof style[keys[0]] === 'string' && style[keys[0]].length > 0",
    };
    format!(
        "(() => {{ const result = render({h}, {}); const style = result.props.style; const keys = Object.keys(style); trace.push('target:' + result.target, 'count:' + keys.length, 'key:' + ({key_check}), 'value:' + ({value_check})); if (keys.length !== {expected_count}) throw Error('style entry count'); return result; }})()",
        style_argument(binding)
    )
}

fn held_projection(shape: StyleShape, binding: Binding, h: bool, key: bool) -> String {
    let value = if key {
        "Object.keys(style)[0].slice(2)".to_string()
    } else {
        format!("style[{:?}]", shape.key(h))
    };
    format!(
        "(() => {{ const result = render({h}, {}); const style = result.props.style; return {{ props: {{ className: {value} }} }}; }})()",
        style_argument(binding)
    )
}
