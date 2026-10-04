use super::{Plan, plan};
use crate::{ExtractOption, ResolvedModule};

#[test]
fn an_import_is_data_when_the_build_knows_its_value_whole() {
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/src/tokens.ts".to_string(),
            code: "export const RED = 'red';\nexport const LIST = [1, 2];\nexport const mutated = { a: 1 };\nmutated.a = 2;\nexport const computed = Date.now();\nexport function twice(n) { return n * 2; }".to_string(),
        })
    };
    let option = ExtractOption::default();
    let plan_of = |code: &str, is_static: &dyn Fn(&str) -> bool| {
        plan(
            &format!("import {{ css }} from '@devup-ui/react';\n{code}"),
            "/src/a.css.ts",
            &option,
            Some(&resolver),
            is_static,
        )
    };
    let no = |_: &str| false;
    for code in [
        "import { RED } from './tokens';\nexport const a = css({ color: RED });",
        "import { RED, LIST } from './tokens';\nexport const a = css({ color: RED, w: LIST[1] });",
        "import { RED as COLOR } from './tokens';\nexport const a = css({ color: COLOR });",
    ] {
        assert_eq!(plan_of(code, &no), Plan::Plain, "{code}");
    }
    for code in [
        "import { mutated } from './tokens';\nexport const a = css({ ...mutated });",
        "import { computed } from './tokens';\nexport const a = css({ w: computed });",
        "import { twice } from './tokens';\nexport const a = css({ w: twice(2) });",
        "import { missing } from './tokens';\nexport const a = css({ w: missing });",
        "import * as tokens from './tokens';\nexport const a = css({ w: tokens.RED });",
    ] {
        assert_eq!(plan_of(code, &no), Plan::Run, "{code}");
    }
    let classes = |name: &str| name == "card";
    assert_eq!(
        plan_of(
            "import { card } from './tokens';\nexport const a = css(card, { p: 1 });",
            &classes
        ),
        Plan::Plain
    );
    assert_eq!(
        plan_of(
            "import { card, other } from './tokens';\nexport const a = css(card, other);",
            &classes
        ),
        Plan::Run
    );
}
