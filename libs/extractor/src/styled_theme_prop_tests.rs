use std::collections::HashMap;

use crate::{ExtractOption, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use insta::assert_snapshot;
use rstest::rstest;
use serial_test::serial;

/// Emotion and styled-components imports aliased to Devup UI, as the plugins
/// alias them by default
fn library_option() -> ExtractOption {
    ExtractOption {
        import_aliases: HashMap::from([
            ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]),
        ..ExtractOption::default()
    }
}

/// The code and CSS `source` compiles to, or the build error it reports
fn compile(source: &str) -> String {
    reset_class_map();
    reset_file_map();
    match extract("theme.tsx", source, library_option()) {
        Ok(output) => {
            let mut styles: Vec<String> = output
                .styles
                .iter()
                .map(|style| format!("{style:?}"))
                .collect();
            styles.sort();
            format!("{}\n{}", output.code, styles.join("\n"))
        }
        Err(error) => error.to_string(),
    }
}

#[rstest]
#[case::template_read(
    "template_read",
    "import styled from 'styled-components';\nconst B = styled.div`color: ${(p) => p.theme.primary};`;\nexport const A = () => <B theme={{ primary: 'blue' }} />;"
)]
#[case::destructured_read(
    "destructured_read",
    "import styled from '@emotion/styled';\nconst B = styled.div`color: ${({ theme }) => theme.colors.line};`;\nexport const A = () => <B theme={{ colors: { line: 'blue' } }} />;"
)]
#[case::object_read(
    "object_read",
    "import styled from '@emotion/styled';\nconst B = styled('div')({ color: (p) => p.theme.brand });\nexport const A = () => <B theme={{ brand: 'blue' }} />;"
)]
#[case::extended_read(
    "extended_read",
    "import styled from 'styled-components';\nconst B = styled.div`color: ${(p) => p.theme.primary};`;\nconst D = styled(B)`margin: 0;`;\nexport const A = () => <D theme={{ primary: 'blue' }} />;"
)]
#[case::no_theme_read(
    "no_theme_read",
    "import styled from 'styled-components';\nconst C = styled.div`color: red;`;\nexport const A = () => <C theme={{ primary: 'blue' }} />;"
)]
#[case::theme_provider(
    "theme_provider",
    "import styled, { ThemeProvider } from 'styled-components';\nconst B = styled.div`color: ${(p) => p.theme.primary};`;\nexport const A = () => <ThemeProvider theme={{ primary: 'blue' }}><B /></ThemeProvider>;"
)]
#[case::plain_component(
    "plain_component",
    "const B = () => <div />;\nexport const A = () => <B theme={{ primary: 'blue' }} />;"
)]
#[serial]
fn styled_theme_prop(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile(code));
}
