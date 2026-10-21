use super::*;

#[derive(Clone, Copy, Debug)]
enum Binding {
    Alias,
    Namespace,
    ShadowAlias,
    ShadowNamespace,
}

impl Binding {
    fn imports(self) -> &'static str {
        match self {
            Self::Alias | Self::ShadowAlias => "import { Box as t, css as i } from '@devup-ui/react';",
            Self::Namespace | Self::ShadowNamespace => "import { Box as t } from '@devup-ui/react'; import * as ui from '@devup-ui/react';",
        }
    }

    fn parameter(self) -> &'static str {
        match self {
            Self::Alias | Self::Namespace => "unused",
            Self::ShadowAlias => "i",
            Self::ShadowNamespace => "ui",
        }
    }

    fn call(self) -> &'static str {
        match self {
            Self::Alias | Self::ShadowAlias => "i({ overflow: 'hidden' })",
            Self::Namespace | Self::ShadowNamespace => "ui.css({ overflow: 'hidden' })",
        }
    }

    fn argument(self, truthy: bool) -> String {
        let helper = format!("rules => {{ trace.push('own:' + rules.overflow); return {}; }}", if truthy { "'local'" } else { "''" });
        match self {
            Self::Alias | Self::Namespace => "void 0".into(),
            Self::ShadowAlias => helper,
            Self::ShadowNamespace => format!("{{ css: {helper} }}"),
        }
    }

    fn shadow(self) -> bool {
        matches!(self, Self::ShadowAlias | Self::ShadowNamespace)
    }
}

fn extract_fixture(source: &str, single_css: bool, active: bool) -> ExtractOutput {
    extract(
        "/src/W70h-native.js",
        source,
        ExtractOption {
            single_css,
            import_aliases: if active {
                HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)])
            } else {
                HashMap::new()
            },
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{source}\n{error}"))
}

fn observe(output: &ExtractOutput, invocation: &str, single_css: bool) -> (Vec<String>, Vec<String>) {
    let observed = render_observation(output, invocation, (!single_css).then_some("/src/W70h-native.js"));
    let (trace, mut declarations): (Vec<String>, Vec<String>) = serde_json::from_str(&observed).unwrap();
    // These fixtures have distinct CSS properties, never competing declarations.
    declarations.sort();
    (trace, declarations)
}

fn assert_own_styles(output: &ExtractOutput, binding: Binding) {
    let own = output.styles.iter().filter(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "overflow" && style.value() == "hidden")).count();
    assert_eq!(own > 0, !binding.shadow(), "{:?}", output.styles);
}

const CLSX: &str = "(...parts) => { trace.push('clsx'); return parts.map(part => typeof part === 'string' ? part : Object.keys(part).filter(key => part[key]).join(' ')).join(' '); }";
