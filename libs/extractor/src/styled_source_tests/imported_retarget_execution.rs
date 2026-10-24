use super::modules;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn imported_captured_template_retargets_when_complete_modules_execute(
    #[values(false, true)] single_css: bool,
) {
    // Given: the exact three authored modules already observed on current and main.
    let _state = crate::test_state::TestStateGuard::new();
    let outputs = modules::compile(
        &[
            (
                "/edge/base.js",
                "import {styled} from '@devup-ui/react';export const Dyn=styled.button`font-size:${p=>p.big?'20px':'10px'};`;",
            ),
            ("/edge/forward.js", "export {Dyn} from './base.js';"),
            (
                "/edge/entry.js",
                "import {styled} from '@devup-ui/react';import {jsx} from '@emotion/react';import * as UI from './forward.js';export const W=UI.Dyn.withComponent('a');export const App=()=>jsx(W,{big:true,ref:7,children:'child'});",
            ),
        ],
        single_css,
    );
    // When: the complete emitted ESM graph evaluates and its actual renderer executes.
    let actual = modules::execute(
        &outputs,
        "import {App} from '/edge/entry.js';const element=render(App());globalThis.observed={target:element.target,ref:element.props.ref,children:element.props.children,values:Object.values(element.props.style)};",
    );
    // Then: retargeting preserves the defining callback's value, target, ref and children.
    assert_eq!(
        actual,
        serde_json::json!({"target":"a","ref":7,"children":"child","values":["20px"]})
    );
}
