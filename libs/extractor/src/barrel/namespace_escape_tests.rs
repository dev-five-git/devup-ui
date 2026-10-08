use super::{NS, Project, namespace_code, namespace_error};
use crate::barrel::{Barreled, rewrite};
use serial_test::serial;

#[test]
#[serial]
fn runtime_namespace_escapes_are_rejected_when_used_whole() {
    for usage in [
        "consume(Devup)",
        "Object.keys(Devup)",
        "function f() { return Devup }",
        "const value = { ns: Devup }",
        "const value = { Devup }",
        "const value = [Devup]",
        "export default Devup",
        "export default { Devup }",
        "const value = typeof Devup",
        "let D = Devup; consume(D)",
        "var D = Devup; consume(D)",
        "const D = Devup; spyOn(D, 'css')",
        "const D = Devup; const E = D; consume(E)",
        "const D = Devup; export default D",
        "function f() { const D = Devup; return D }",
        "const E = D; const D = Devup; consume(E)",
        "export { D }; const D = Devup",
        "consume((Devup))",
        "consume(Devup as object)",
    ] {
        let source = format!("{NS}{usage}\n");

        let error = namespace_error(&source);

        assert!(error.contains("test.tsx:2:"), "{usage}: {error}");
        assert!(
            error.contains("cannot use") && error.contains("read its members by name"),
            "{usage}: {error}"
        );
    }
}

#[test]
#[serial]
fn safe_namespace_reads_preserve_semantic_identity() {
    let source = format!(
        "{NS}const D = Devup; const E = D;\nconst {{ css: c }} = E;\nexport const a = c({{ color: 'red' }});\nexport const b = D.css({{ color: 'blue' }});\nexport type N = typeof Devup;\nexport type A = typeof D;\nexport function f(Devup) {{ return spyOn(Devup, 'css') }}\nexport function g(D) {{ return D }}\nexport {{ Devup as UI }};\n"
    );

    let code = namespace_code(&source);

    assert!(!code.contains("c({") && !code.contains("D.css("), "{code}");
    assert!(code.contains("spyOn(Devup, \"css\")"), "{code}");
    assert!(code.contains("return D"), "{code}");
    assert!(
        code.contains("typeof Devup") && code.contains("typeof D"),
        "{code}"
    );
    assert!(code.contains("export * as UI"), "{code}");
}

#[test]
#[serial]
fn type_only_and_unrelated_namespaces_remain_unchanged() {
    for source in [
        "import type * as Devup from '@devup-ui/react'; export type N = typeof Devup;",
        "import * as Devup from 'other'; spyOn(Devup, 'css');",
        "import * as Devup from '@devup-ui/react'; function f() { const Devup = {}; return Devup }",
    ] {
        let result = rewrite(source, "test.tsx", "@devup-ui/react", None);

        assert!(matches!(result, Barreled::Unchanged));
    }
}

#[test]
#[serial]
fn reexported_namespaces_are_rejected_when_their_runtime_value_escapes() {
    for (index, (barrel, import)) in [
        (
            "export { css } from '@devup-ui/react'",
            "import * as Devup from './ui'",
        ),
        (
            "export * as Devup from '@devup-ui/react'",
            "import { Devup } from './ui'",
        ),
        (
            "import * as D from '@devup-ui/react'; export default D",
            "import Devup from './ui'",
        ),
        (
            "import * as D from '@devup-ui/react'; export const Devup = D",
            "import { Devup } from './ui'",
        ),
        (
            "export * as Devup from './inner'",
            "import { Devup } from './ui'",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let source = format!("{import}\nspyOn(Devup, 'css')\n");
        let project = Project::new(
            &format!("escape-{index}"),
            &[
                ("ui.ts", barrel),
                ("inner.ts", "export { css } from '@devup-ui/react'"),
                ("app.tsx", &source),
            ],
        );

        let error = project.error("app.tsx");

        assert!(error.contains("app.tsx:2:7:"), "{error}");
        assert!(
            error.contains("cannot use") && error.contains("import"),
            "{error}"
        );
    }
}

#[test]
#[serial]
fn default_namespace_import_is_rejected_when_passed_to_runtime() {
    let source = "import Devup from '@devup-ui/react'\nspyOn(Devup, 'css')\n";

    let error = namespace_error(source);

    assert!(
        error.contains("test.tsx:2:7: `Devup` cannot use `Devup`"),
        "{error}"
    );
}
