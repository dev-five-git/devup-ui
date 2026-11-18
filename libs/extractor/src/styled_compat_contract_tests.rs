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
    match extract("compat.tsx", source, library_option()) {
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
#[case::emotion_target(
    "emotion_target",
    "import styled from '@emotion/styled';\nexport const Card = styled('div', { label: 'card', target: 'e1card' })`color: red;`;\nexport const Wide = styled(Card)`width: 100%;`;"
)]
#[case::component_id(
    "component_id",
    "import styled from 'styled-components';\nexport const Card = styled.div.withConfig({ displayName: 'Card', componentId: 'sc-card' })`color: red;`;"
)]
#[case::identity_errors(
    "identity_errors",
    "import styled from '@emotion/styled';\nimport sc from 'styled-components';\nexport const A = styled('div', { target: make() })`color: red;`;\nexport const B = sc.div.withConfig({ componentId: 1 })`color: red;`;\nexport const C = sc.div.withConfig({ componentId: null })`color: red;`;\nexport const D = sc.div.withConfig({ componentId: id })`color: red;`;\nexport const E = sc.div.withConfig({ displayName: name, componentId: 'e' })`color: red;`;\nexport const F = sc.div.withConfig({ displayName: 5, componentId: 'f' })`color: red;`;\nexport const G = styled('div', { target: [...parts] })`color: red;`;\nexport const H = styled('div', { target: { toString: () => 'h' } })`color: red;`;\nexport const I = styled('div', { target: { ...parts } })`color: red;`;"
)]
#[case::emotion_target_values(
    "emotion_target_values",
    "import styled from '@emotion/styled';\nexport const Empty = styled('div', { target: '' })`color: red;`;\nexport const Blank = styled('div', { target: ' \\t ' })`color: red;`;\nexport const Words = styled('div', { target: ' two  words ' })`color: red;`;\nexport const Wrapped = styled(Base, { target: '' })`color: red;`;\nexport const Num = styled('div', { target: 0 })`color: red;`;\nexport const Null = styled('div', { target: null })`color: red;`;\nexport const Bool = styled('div', { target: false })`color: red;`;\nexport const Missing = styled('div', { target: undefined })`color: red;`;\nexport const Nan = styled('div', { target: NaN })`color: red;`;\nexport const Inf = styled('div', { target: Infinity })`color: red;`;\nexport const List = styled('div', { target: ['a', , null, undefined, 1, ['b', 'c']] })`color: red;`;\nexport const Plain = styled('div', { target: {} })`color: red;`;\nexport const Template = styled('div', { target: `tpl` as const })`color: red;`;"
)]
#[case::emotion_target_shadowed(
    "emotion_target_shadowed",
    "import styled from '@emotion/styled';\nconst undefined = 'x';\nexport const A = styled('div', { target: undefined })`color: red;`;"
)]
#[case::component_id_values(
    "component_id_values",
    "import styled from 'styled-components';\nexport const Empty = styled.div.withConfig({ componentId: '' })`color: red;`;\nexport const Other = styled.div.withConfig({ componentId: '' })`color: blue;`;\nexport const Blank = styled.div.withConfig({ componentId: '  ' })`color: red;`;\nexport const Words = styled.div.withConfig({ componentId: 'two words' })`color: red;`;\nexport const Wrapped = styled(Base).withConfig({ componentId: '' })`color: red;`;\nexport const Missing = styled.div.withConfig({ componentId: undefined })`color: red;`;\nexport const Last = styled.div.withConfig({ componentId: 'first' }).withConfig({ componentId: 'last' })`color: red;`;"
)]
#[case::component_id_display_name(
    "component_id_display_name",
    "import styled from 'styled-components';\nexport const Named = styled.div.withConfig({ displayName: 'My Card!', componentId: 'cid' })`color: red;`;\nexport const EmptyName = styled.div.withConfig({ displayName: '', componentId: 'cid' })`color: red;`;\nexport const FalsyName = styled.div.withConfig({ displayName: 0, componentId: 'cid' })`color: red;`;\nexport const NoName = styled.div.withConfig({ displayName: undefined, componentId: 'cid' })`color: red;`;\nexport const EmptyId = styled.div.withConfig({ displayName: name, componentId: '' })`color: red;`;"
)]
#[case::component_id_extension(
    "component_id_extension",
    "import styled from 'styled-components';\nexport const A = styled.div.withConfig({ componentId: 'idA' })`color: red;`;\nexport const B = styled(A)`color: blue;`;\nexport const C = styled(A).withConfig({ componentId: '' })`color: blue;`;\nexport const E = styled.div.withConfig({ componentId: '' })`color: red;`;\nexport const D = styled(E).withConfig({ componentId: 'idD' })`color: blue;`;"
)]
#[case::template_label(
    "template_label",
    "import { css } from '@emotion/react';\nexport const named = css`label: card; color: red;`;"
)]
#[case::default_props(
    "default_props",
    "import styled from 'styled-components';\nexport const Tone = styled.div`color: ${(p) => p.$tone};`;\nTone.defaultProps = { $tone: 'red' };\nexport const Plain = () => null;\nPlain.defaultProps = { tone: 'red' };\nTone.displayName = 'Tone';"
)]
#[case::global_class(
    "global_class",
    "import { Global, css } from '@emotion/react';\nconst reset = css`body { margin: 0; } @media print { html { color: black; } }`;\nexport const App = () => <Global styles={reset} />;"
)]
#[case::global_own_declarations(
    "global_own_declarations",
    "import { Global, css } from '@emotion/react';\nconst red = css({ color: 'red' });\nconst hover = css`&:hover { color: blue; }`;\nconst print = css({ '@media print': { color: 'black' } });\nconst text = css({ typography: 'heading' });\nexport const App = () => <><Global styles={red} /><Global styles={hover} /><Global styles={print} /><Global styles={text} /></>;"
)]
#[case::global_array(
    "global_array",
    "import { Global } from '@emotion/react';\nconst base = { html: { color: 'red' } };\nconst more = [{ p: { margin: 0 } }];\nexport const App = ({ on }) => <Global styles={[{ body: { margin: 0 } }, base, false, null, undefined, , more]} />;"
)]
#[case::global_array_spread(
    "global_array_spread",
    "import { Global } from '@emotion/react';\nexport const App = ({ rest }) => <Global styles={[{ body: { margin: 0 } }, ...rest]} />;"
)]
#[case::global_theme(
    "global_theme",
    "import { Global } from '@emotion/react';\nexport const App = () => <Global styles={(theme) => ({ body: { color: theme.colors.text, padding: theme.space[2] } })} />;\nexport const Plain = () => <Global styles={function (theme) { return [{ a: { color: theme.link } }]; }} />;"
)]
#[case::global_theme_call(
    "global_theme_call",
    "import { Global } from '@emotion/react';\nexport const App = () => <Global styles={(theme) => ({ body: { color: theme.pick() } })} />;"
)]
#[serial]
fn styled_compat_contract(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile(code));
}
