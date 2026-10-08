use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use css::class_map::reset_class_map;
use css::file_map::reset_file_map;
use serial_test::serial;

use crate::{
    ExtractOption, ExtractOutput, ModuleResolver, ResolvedModule, extract, extract_with_modules,
    has_devup_ui_with,
};

#[path = "namespace_escape_tests.rs"]
mod namespace_escape_tests;

/// A project of real files on disk, removed when dropped
struct Project {
    root: PathBuf,
}

impl Project {
    fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("devup-barrel-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (path, code) in files {
            let file = root.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, code).unwrap();
        }
        Self { root }
    }

    fn path(&self, name: &str) -> String {
        self.root.join(name).to_string_lossy().replace('\\', "/")
    }

    fn resolver(&self) -> impl Fn(&str, &str) -> Option<ResolvedModule> + use<> {
        let root = self.root.clone();
        move |specifier, importer| {
            let base = if specifier.starts_with('.') {
                normalize(&Path::new(importer).parent().unwrap().join(specifier))
            } else {
                root.join("node_modules").join(specifier)
            };
            let mut candidates = vec![base.clone()];
            for extension in ["ts", "tsx", "js"] {
                candidates.push(PathBuf::from(format!("{}.{extension}", base.display())));
                candidates.push(base.join(format!("index.{extension}")));
            }
            candidates
                .into_iter()
                .find(|candidate| candidate.is_file())
                .map(|path| ResolvedModule {
                    code: fs::read_to_string(&path).unwrap(),
                    path: path.to_string_lossy().replace('\\', "/"),
                })
        }
    }

    fn extract(&self, name: &str) -> Result<ExtractOutput, String> {
        reset_class_map();
        reset_file_map();
        let filename = self.path(name);
        let code = fs::read_to_string(&filename).unwrap();
        extract_with_modules(
            &filename,
            &code,
            ExtractOption::default(),
            false,
            &self.resolver(),
        )
        .map_err(|error| error.to_string())
    }

    fn code(&self, name: &str) -> String {
        self.extract(name).unwrap().code
    }

    fn error(&self, name: &str) -> String {
        self.extract(name).unwrap_err()
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component),
        }
    }
    normalized
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const APP: &str = "import { Box } from './ui'\nexport const a = <Box bg=\"red\" />\n";

fn assert_compiled(code: &str) {
    assert!(code.contains("<div className="), "{code}");
    assert!(!code.contains("<Box"), "{code}");
}

#[test]
#[serial]
fn single_barrel_compiles_like_the_package() {
    let project = Project::new(
        "single",
        &[
            (
                "ui/index.ts",
                "export { Box, css } from '@devup-ui/react'\n",
            ),
            ("app.tsx", APP),
        ],
    );
    let output = project.extract("app.tsx").unwrap();
    assert_compiled(&output.code);
    assert_eq!(output.dependencies, vec![project.path("ui/index.ts")]);
    assert_ne!(output.styles.len(), 0);
}

#[test]
#[serial]
fn barrel_compiles_the_same_as_the_package() {
    let project = Project::new(
        "same",
        &[
            ("ui.ts", "export { Box } from '@devup-ui/react'\n"),
            ("app.tsx", APP),
            (
                "direct.tsx",
                "import { Box } from '@devup-ui/react'\nexport const a = <Box bg=\"red\" />\n",
            ),
        ],
    );
    assert_eq!(project.code("app.tsx"), project.code("direct.tsx"));
}

#[test]
#[serial]
fn chained_barrels_are_followed() {
    let project = Project::new(
        "chain",
        &[
            ("ui/index.ts", "export * from './components'\n"),
            (
                "ui/components/index.ts",
                "export { Box } from './box'\nexport { Text } from '@devup-ui/react'\n",
            ),
            (
                "ui/components/box.ts",
                "export { Box } from '@devup-ui/react'\n",
            ),
            ("app.tsx", APP),
        ],
    );
    let output = project.extract("app.tsx").unwrap();
    assert_compiled(&output.code);
    assert_eq!(
        output.dependencies,
        vec![
            project.path("ui/components/box.ts"),
            project.path("ui/components/index.ts"),
            project.path("ui/index.ts"),
        ]
    );
}

#[test]
#[serial]
fn export_star_of_the_package_is_followed() {
    let project = Project::new(
        "star",
        &[
            (
                "ui/index.ts",
                "export * from '@devup-ui/react'\nexport * from './other'\n",
            ),
            ("ui/other.ts", "export const other = 1\nexport {}\n"),
            (
                "app.tsx",
                "import { Box, css } from './ui'\nimport { other } from './ui'\nexport const a = <Box bg=\"red\" className={css({ color: 'blue' })} />\nexport const b = other\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert_compiled(&code);
    assert!(code.contains("import { other } from \"./ui\""), "{code}");
    assert!(!code.contains("css("), "{code}");
}

#[test]
#[serial]
fn renamed_exports_keep_the_importers_names() {
    let project = Project::new(
        "renamed",
        &[
            (
                "ui.ts",
                "export { Box as Container, css as style, styled as s } from '@devup-ui/react'\n",
            ),
            (
                "app.tsx",
                "import { Container as C, style, s } from './ui'\nexport const a = <C bg=\"red\" />\nexport const b = style({ color: 'blue' })\nexport const D = s('div', { color: 'green' })\nexport const d = <D />\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert_compiled(&code);
    assert!(!code.contains("style("), "{code}");
}

#[test]
#[serial]
fn compat_entry_is_followed() {
    let project = Project::new(
        "compat",
        &[
            (
                "ui.ts",
                "export { useTheme } from '@devup-ui/react/compat'\n",
            ),
            (
                "app.tsx",
                "import { useTheme } from './ui'\nexport const t = useTheme\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(code.contains("@devup-ui/react/compat"), "{code}");
    assert!(!code.contains("./ui"), "{code}");
}

#[test]
#[serial]
fn imports_of_other_things_stay() {
    let project = Project::new(
        "mixed",
        &[
            (
                "ui.ts",
                "import { Box as B } from '@devup-ui/react'\nexport { B }\nexport const helper = 1\nexport type Props = {}\nexport default function Page() {}\n",
            ),
            (
                "app.tsx",
                "import Page, { B as Box, helper, type Props } from './ui'\nexport const a = <Box bg=\"red\" />\nexport const b = [Page, helper]\nexport const c: Props = {}\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert_compiled(&code);
    assert!(
        code.contains("import Page, { helper, type Props } from \"./ui\""),
        "{code}"
    );
}

#[test]
#[serial]
fn default_exports_of_the_package_are_followed() {
    let project = Project::new(
        "default",
        &[
            (
                "ui.ts",
                "import { Box } from '@devup-ui/react'\nexport default Box\n",
            ),
            (
                "app.tsx",
                "import UI from './ui'\nexport const a = <UI bg=\"red\" />\n",
            ),
        ],
    );
    assert!(project.code("app.tsx").contains("<div className="));
}

#[test]
#[serial]
fn namespace_re_exports_are_followed() {
    let project = Project::new(
        "namespace-reexport",
        &[
            ("ui.ts", "export * as Devup from '@devup-ui/react'\n"),
            (
                "app.tsx",
                "import { Devup } from './ui'\nexport const a = <Devup.Box bg=\"red\" />\n",
            ),
        ],
    );
    assert_compiled(&project.code("app.tsx"));
}

#[test]
#[serial]
fn namespace_of_a_barrel_is_read_by_member() {
    let project = Project::new(
        "barrel-namespace",
        &[
            (
                "ui.ts",
                "export { Box, css } from '@devup-ui/react'\nexport const helper = 1\n",
            ),
            (
                "app.tsx",
                "import * as UI from './ui'\nexport const a = <UI.Box bg=\"red\">{UI.helper}</UI.Box>\nexport const { css: c, helper } = { css: 1, helper: 2 }\nexport const d = UI['css']({ color: 'blue' })\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(code.contains("<div className="), "{code}");
    assert!(code.contains("UI.helper"), "{code}");
}

#[test]
#[serial]
fn destructured_barrel_namespace_is_read_from_the_package() {
    let project = Project::new(
        "barrel-destructure",
        &[
            ("ui.ts", "export { Box, css } from '@devup-ui/react'\n"),
            (
                "app.tsx",
                "import * as UI from './ui'\nconst { Box: B, css } = UI\nexport const a = <B bg=\"red\" />\nexport const b = css({ color: 'blue' })\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert_compiled(&code);
    assert!(!code.contains("= UI"), "{code}");
}

#[test]
#[serial]
fn destructure_leaves_what_the_barrel_declares() {
    let project = Project::new(
        "barrel-destructure-own",
        &[
            (
                "ui.ts",
                "export { Box } from '@devup-ui/react'\nexport const helper = 1\n",
            ),
            (
                "app.tsx",
                "import * as UI from './ui'\nconst { helper } = UI\nexport const a = helper\n",
            ),
        ],
    );
    assert!(project.code("app.tsx").contains("{ helper } = UI"));
}

#[test]
#[serial]
fn unresolvable_barrel_is_a_located_error_where_used() {
    let project = Project::new(
        "unresolvable",
        &[
            (
                "ui.ts",
                "export * from './missing'\nexport { Box } from '@devup-ui/react'\n",
            ),
            (
                "app.tsx",
                "import { Foo, Box } from './ui'\nexport const a = 1\nexport const b = <Foo />\n",
            ),
            (
                "unused.tsx",
                "import { Foo } from './ui'\nexport const a = 1\n",
            ),
            (
                "named.ts",
                "export { Foo } from './missing'\nexport { Box } from '@devup-ui/react'\n",
            ),
            (
                "app-named.tsx",
                "import { Foo } from './named'\nexport const b = <Foo />\n",
            ),
        ],
    );
    let error = project.error("app.tsx");
    assert!(
        error.contains("app.tsx:3:19: `Foo` cannot use `./ui` at build time"),
        "{error}"
    );
    assert!(error.contains("`./missing` cannot be read"), "{error}");
    assert_eq!(
        project.code("unused.tsx"),
        "import { Foo } from './ui'\nexport const a = 1\n"
    );
    assert!(
        project
            .error("app-named.tsx")
            .contains("`./missing` cannot be read")
    );
}

#[test]
#[serial]
fn unresolvable_modules_unrelated_to_the_package_stay() {
    let project = Project::new(
        "unrelated",
        &[
            (
                "ui.ts",
                "export * from './missing'\nexport { x } from './missing'\n",
            ),
            (
                "app.tsx",
                "import { Foo, x } from './ui'\nexport const a = [Foo, x]\n",
            ),
        ],
    );
    assert!(
        project
            .code("app.tsx")
            .contains("import { Foo, x } from './ui'")
    );
}

#[test]
#[serial]
fn namespaces_of_modules_are_followed_or_reported() {
    let project = Project::new(
        "namespace-module",
        &[
            ("inner.ts", "export { Box } from '@devup-ui/react'\n"),
            ("plain.ts", "export const x = 1\n"),
            (
                "ui.ts",
                "export * as inner from './inner'\nexport * as plain from './plain'\n",
            ),
            (
                "app.tsx",
                "import { plain } from './ui'\nexport const a = plain.x\n",
            ),
            (
                "bad.tsx",
                "import { inner } from './ui'\nexport const a = <inner.Box />\n",
            ),
        ],
    );
    assert!(
        project
            .code("app.tsx")
            .contains("import { plain } from './ui'")
    );
    assert!(
        project
            .error("bad.tsx")
            .contains("a namespace of modules re-exporting `@devup-ui/react`")
    );
}

#[test]
#[serial]
fn cycles_between_barrels_end() {
    let project = Project::new(
        "cycle",
        &[
            (
                "a.ts",
                "export * from './b'\nexport { Box } from '@devup-ui/react'\n",
            ),
            ("b.ts", "export * from './a'\n"),
            (
                "app.tsx",
                "import { Box, nothing } from './b'\nexport const a = <Box bg=\"red\" />\nexport const n = nothing\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert_compiled(&code);
    assert!(code.contains("import { nothing } from \"./b\""), "{code}");
}

#[test]
#[serial]
fn files_that_cannot_lead_to_the_package_are_not_read_further() {
    let project = Project::new(
        "plain",
        &[
            ("plain.ts", "export const x = 1\n"),
            (
                "app.tsx",
                "import { x } from './plain'\nimport './side-effect'\nimport type { T } from './plain'\nimport {} from './plain'\nexport const a = x\n",
            ),
        ],
    );
    let output = project.extract("app.tsx").unwrap();
    assert_eq!(output.styles.len(), 0);
    assert_eq!(output.dependencies, Vec::<String>::new());
}

#[test]
#[serial]
fn type_only_and_attribute_imports_are_left() {
    let project = Project::new(
        "types",
        &[
            ("ui.ts", "export { Box } from '@devup-ui/react'\n"),
            (
                "app.tsx",
                "import type { Box } from './ui'\nimport data from './ui' with { type: 'json' }\nexport type A = typeof Box\nexport const d = data\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(code.contains("./ui"), "{code}");
}

#[test]
#[serial]
fn export_forms_that_are_not_re_exports_are_declarations() {
    let project = Project::new(
        "declarations",
        &[
            (
                "ui.ts",
                "import { Box } from '@devup-ui/react'\nexport type { Box as BoxType } from '@devup-ui/react'\nexport type * from '@devup-ui/react'\nexport { type Box as T }\nexport enum E { A }\nexport default (() => Box)\n",
            ),
            (
                "app.tsx",
                "import D, { BoxType, E, T } from './ui'\nexport const a = [D, BoxType, E, T]\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(
        code.contains("import D, { BoxType, E, T } from './ui'"),
        "{code}"
    );
}

#[test]
#[serial]
fn parse_failures_are_left_alone() {
    let project = Project::new(
        "broken",
        &[
            ("ui.ts", "export { Box } from '@devup-ui/react'\n"),
            ("app.tsx", "import { Box } from './ui'\nconst = <\n"),
        ],
    );
    assert_eq!(
        project.code("app.tsx"),
        "import { Box } from './ui'\nconst = <\n"
    );
}

#[test]
#[serial]
fn vanilla_extract_stylesheets_are_left_alone() {
    let project = Project::new(
        "vanilla",
        &[
            ("ui.ts", "export { style } from '@devup-ui/react'\n"),
            (
                "a.css.ts",
                "import { style } from '@vanilla-extract/css'\nexport const a = style({ color: 'red' })\n",
            ),
        ],
    );
    assert!(project.extract("a.css.ts").is_ok());
}

fn namespace_error(code: &str) -> String {
    reset_class_map();
    reset_file_map();
    extract("test.tsx", code, ExtractOption::default())
        .unwrap_err()
        .to_string()
}

fn namespace_code(code: &str) -> String {
    reset_class_map();
    reset_file_map();
    extract("test.tsx", code, ExtractOption::default())
        .unwrap()
        .code
}

const NS: &str = "import * as Devup from '@devup-ui/react'\n";

#[test]
#[serial]
fn namespace_members_compile_by_name() {
    let code = namespace_code(&format!(
        "{NS}export const a = <Devup.Box bg=\"red\">x</Devup.Box>\nexport const b = Devup['css']({{ color: 'blue' }})\nexport const c = Devup[`css`]({{ color: 'green' }})\nexport const d = Devup.css({{ color: 'blue' }})\n"
    ));
    assert!(code.contains("<div className="), "{code}");
    assert!(!code.contains("css("), "{code}");
    assert!(!code.contains("</Devup"), "{code}");
}

#[test]
#[serial]
fn destructured_namespace_compiles_where_declared() {
    let code = namespace_code(&format!(
        "{NS}const {{ css, Box: B }} = Devup\nexport const a = css({{ color: 'red' }})\nexport const b = <B bg=\"red\" />\nexport function f() {{\n  const {{ css: local }} = Devup\n  return local({{ color: 'blue' }})\n}}\nconst g = 1, {{ keyframes }} = Devup, h = 2\nexport const k = [g, h, keyframes({{ from: {{ opacity: 0 }} }})]\nconst m = 1, {{ globalCss }} = Devup\nglobalCss({{ body: {{ margin: 0 }} }})\n"
    ));
    assert!(code.contains("<div className="), "{code}");
    assert!(!code.contains("= Devup"), "{code}");
}

#[test]
#[serial]
fn namespace_names_do_not_collide() {
    let code = namespace_code(&format!(
        "{NS}const Devup$css = 1\nexport const a = Devup.css({{ color: 'red' }})\nexport const b = Devup.css({{ color: 'blue' }})\nexport const c = Devup$css\n"
    ));
    assert!(code.contains("const Devup$css = 1"), "{code}");
    assert!(code.contains("export const c = Devup$css;"), "{code}");
}

#[test]
#[serial]
fn default_import_of_the_package_reads_members() {
    let code = namespace_code(
        "import Devup from '@devup-ui/react'\nexport const a = <Devup.Box bg=\"red\" />\n",
    );
    assert!(code.contains("<div className="), "{code}");
}

#[test]
#[serial]
fn stylex_namespace_is_left() {
    let code = namespace_code(&format!(
        "{NS}const {{ stylex }} = Devup\nexport const a = stylex\n"
    ));
    assert!(code.contains("{ stylex } = Devup"), "{code}");
}

#[test]
#[serial]
fn namespace_members_read_as_values_are_located_errors() {
    let error = namespace_error(&format!("{NS}export const a = [Devup.css]\n"));
    assert!(error.contains("test.tsx:2:19:"), "{error}");
    let error = namespace_error(&format!("{NS}export const a = Devup['css']\n"));
    assert!(error.contains("test.tsx:2:18:"), "{error}");
    let error = namespace_error(&format!(
        "{NS}const f = (fn) => fn\nexport const a = f(Devup.css)\n"
    ));
    assert!(error.contains("test.tsx:3:20:"), "{error}");
}

#[test]
#[serial]
fn namespace_read_whole_is_a_located_error() {
    for code in [
        "export const key = 'css'\nexport const a = Devup[key]",
        "Devup.css = 1",
        "export const [a] = Devup",
        "export const { ...rest } = Devup",
        "const key = 'css'\nexport const { [key]: a } = Devup",
        "export const { css = 1 } = Devup",
        "export const { css: [a] } = Devup",
        "export const { css } = Devup",
        "const { css, stylex } = Devup\nexport const a = [css, stylex]",
    ] {
        let error = namespace_error(&format!("{NS}{code}\n"));
        assert!(
            error.contains("`Devup` cannot use `") && error.contains("at build time"),
            "{code}: {error}"
        );
    }
}

#[test]
#[serial]
fn binding_reads_of_a_destructured_namespace_keep_their_meaning() {
    for code in [
        "const { css } = Devup\nexport const a = { css }\n",
        "const { css } = Devup\nexport { css }\n",
        "const { css } = Devup\nexport { css as c }\n",
    ] {
        let error = namespace_error(&format!("{NS}{code}"));
        assert!(error.contains("test.tsx:"), "{code}: {error}");
    }
    let error = namespace_error(&format!("{NS}let {{ css }} = Devup\ncss = 1\n"));
    assert!(error.contains("`Devup` cannot use `css`"), "{error}");
}

#[test]
#[serial]
fn namespace_without_a_resolver_or_package_text_is_unchanged() {
    let output = extract(
        "test.tsx",
        "import * as Other from 'other'\nexport const a = Other.x\n",
        ExtractOption::default(),
    )
    .unwrap();
    assert_eq!(output.styles.len(), 0);
}

#[test]
#[serial]
fn barrel_namespace_errors_name_the_member() {
    let project = Project::new(
        "barrel-namespace-errors",
        &[
            (
                "ui.ts",
                "export * from './missing'\nexport { Box } from '@devup-ui/react'\nexport const own = 1\n",
            ),
            (
                "member.tsx",
                "import * as UI from './ui'\nexport const a = UI.nothing\nexport const b = UI.own\n",
            ),
            (
                "destructure.tsx",
                "import * as UI from './ui'\nconst { nothing } = UI\nexport const a = nothing\n",
            ),
            (
                "mixed.tsx",
                "import * as UI from './ui'\nconst { Box, own } = UI\nexport const a = [Box, own]\n",
            ),
        ],
    );
    let error = project.error("member.tsx");
    assert!(error.contains("`UI.nothing` cannot use `./ui`"), "{error}");
    let error = project.error("destructure.tsx");
    assert!(
        error.contains("`{ nothing } = UI` cannot use `./ui`"),
        "{error}"
    );
    let error = project.error("mixed.tsx");
    assert!(
        error.contains("`UI` cannot use `const { Box, own } = UI`")
            || error.contains("`UI` cannot use `{ Box, own } = UI`"),
        "{error}"
    );
}

#[test]
#[serial]
fn unfollowable_default_and_named_imports_report_each_use() {
    let project = Project::new(
        "unfollowable-default",
        &[
            (
                "ui.ts",
                "export { default } from './missing'\nexport { Box } from '@devup-ui/react'\n",
            ),
            (
                "app.tsx",
                "import Page from './ui'\nexport const a = [Page, Page]\n",
            ),
        ],
    );
    let error = project.error("app.tsx");
    assert!(
        error.contains("app.tsx:2:19:") && error.contains("app.tsx:2:25:"),
        "{error}"
    );
}

#[test]
#[serial]
fn has_devup_ui_follows_barrels() {
    let project = Project::new(
        "has",
        &[
            ("ui.ts", "export { Box } from '@devup-ui/react'\n"),
            ("plain.ts", "export const x = 1\n"),
        ],
    );
    let resolver = project.resolver();
    let resolver: &ModuleResolver = &resolver;
    let file = project.path("app.tsx");
    let gate = |file: &str, code: &str, resolver: &ModuleResolver| {
        has_devup_ui_with(
            file,
            code,
            "@devup-ui/react",
            &HashMap::new(),
            Some(resolver),
        )
    };
    assert!(gate(&file, "import { Box } from './ui'", resolver));
    assert!(gate(
        &file,
        "import { Box } from '@devup-ui/react'",
        resolver
    ));
    assert!(!gate(&file, "import { x } from './plain'", resolver));
    assert!(!gate(&file, "const a = 1", resolver));
    assert!(!gate("app.invalid", "import { Box } from './ui'", resolver));
    assert!(gate(
        &file,
        "import { Foo } from './unresolved'\nimport * as Devup from '@devup-ui/react'\nDevup.css",
        resolver
    ));
}

#[test]
#[serial]
fn namespace_of_the_package_inside_a_barrel_namespace_is_followed() {
    let project = Project::new(
        "nested-namespace",
        &[
            (
                "ui.ts",
                "export * as Devup from '@devup-ui/react'\nexport const own = 1\n",
            ),
            (
                "app.tsx",
                "import * as UI from './ui'\nexport const a = <UI.Devup.Box bg=\"red\" />\nexport const b = UI.missing\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(code.contains("<div className="), "{code}");
    assert!(code.contains("UI.missing"), "{code}");
}

#[test]
#[serial]
fn computed_keys_that_are_not_one_literal_read_the_namespace_whole() {
    let error = namespace_error(&format!(
        "{NS}export const a = Devup[`c${{'ss'}}`]({{ color: 'red' }})\n"
    ));
    assert!(error.contains("`Devup` cannot use `Devup[`"), "{error}");
}

#[test]
#[serial]
fn long_code_in_an_error_is_shortened() {
    let error = namespace_error(&format!(
        "{NS}export const a = Devup[('a very long key that goes on and on and on and on and on', 1)]\n"
    ));
    assert!(error.contains("…"), "{error}");
}

#[test]
#[serial]
fn namespace_passed_whole_is_a_located_build_error() {
    let error = namespace_error(&format!("{NS}spyOn(Devup, 'css')\n"));
    assert!(
        error.contains("test.tsx:2:7: `Devup` cannot use `Devup` at build time"),
        "{error}"
    );
    assert!(error.contains("read its members by name"), "{error}");
}

#[test]
#[serial]
fn namespace_styled_in_a_stylesheet_is_compiled_by_the_visitor() {
    reset_class_map();
    reset_file_map();
    let output = extract(
        "test.css.ts",
        "import * as Devup from '@devup-ui/react'\nexport const A = Devup.styled.div({ color: 'red' })\n",
        ExtractOption::default(),
    )
    .unwrap();
    assert!(!output.code.contains("Devup.styled"), "{}", output.code);
    assert_ne!(output.styles.len(), 0);
}

#[test]
fn gate_sees_files_only_extraction_of_aliases_changes() {
    let aliases = HashMap::from([
        (
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        ),
        (
            "@emotion/styled".to_string(),
            crate::ImportAlias::DefaultToNamed("styled".to_string()),
        ),
    ]);
    let gate = |file: &str, code: &str, aliases: &HashMap<String, crate::ImportAlias>| {
        has_devup_ui_with(file, code, "@devup-ui/react", aliases, None)
    };
    assert!(gate(
        "a.tsx",
        "import styled from '@emotion/styled'\nexport const A = styled.div({})",
        &aliases
    ));
    assert!(gate(
        "a.tsx",
        "/** @jsxImportSource @emotion/react */\nexport const a = <div css={{ color: 'red' }} />",
        &aliases
    ));
    assert!(gate(
        "a.tsx",
        "import { css } from '@emotion/react'\nexport const a = <div css={css({})} />",
        &aliases
    ));
    assert!(!gate(
        "a.tsx",
        "export const a = <div className=\"x\" />",
        &aliases
    ));
    assert!(!gate(
        "a.tsx",
        "import styled from '@emotion/styled'",
        &HashMap::new()
    ));
    let jsx_runtime = HashMap::from([
        (
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        ),
        (
            "@emotion/react/jsx-runtime".to_string(),
            crate::ImportAlias::NamedToNamed,
        ),
    ]);
    assert!(gate(
        "a.tsx",
        "export const a = <div css={{ color: 'red' }} />",
        &jsx_runtime
    ));
}

#[test]
#[serial]
fn barrel_aliases_of_package_exports_are_followed_and_compile_themselves() {
    let project = Project::new(
        "alias-barrel",
        &[
            (
                "ui.tsx",
                "import { Box, css, Text } from '@devup-ui/react'\nimport * as Devup from '@devup-ui/react'\nexport const B = Box\nconst c = css\nexport { c as cc, Text }\nexport const D = Devup\nexport const C = Devup.css\nexport const helper = 1, F = Text\n",
            ),
            (
                "app.tsx",
                "import { B, cc, Text, D, helper } from './ui'\nexport const a = <B bg=\"red\"><Text color=\"blue\" /></B>\nexport const b = cc({ color: 'green' })\nexport const c = <D.Flex gap={2} />\nexport const d = helper\n",
            ),
            (
                "member.tsx",
                "import { C } from './ui'\nexport const a = C({ color: 'red' })\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(code.contains("<div className="), "{code}");
    assert!(code.contains("helper"), "{code}");
    assert!(!code.contains("cc("), "{code}");
    assert!(
        project.code("member.tsx").contains("\"a"),
        "{}",
        project.code("member.tsx")
    );
    let barrel = project.code("ui.tsx");
    assert!(
        barrel.contains("export { css as C } from \"@devup-ui/react\""),
        "{barrel}"
    );
    assert!(barrel.contains("export { helper };"), "{barrel}");
}

#[test]
#[serial]
fn barrel_module_exports_what_it_aliases_as_re_exports() {
    let project = Project::new(
        "alias-barrel-compile",
        &[(
            "ui.tsx",
            "import { Box, css, Text } from '@devup-ui/react'\nimport * as Devup from '@devup-ui/react'\nexport const B = Box\nconst c = css\nexport { c as cc, Text, type Props }\nexport default css\nexport const D = Devup\nexport const kept = 1\nexport type Props = {}\n",
        )],
    );
    let code = project.code("ui.tsx");
    for exported in [
        "export { Box as B } from \"@devup-ui/react\"",
        "export { css as cc } from \"@devup-ui/react\"",
        "export { Text } from \"@devup-ui/react\"",
        "export { css as default } from \"@devup-ui/react\"",
        "export * as D from \"@devup-ui/react\"",
        "export { type Props }",
        "export const kept = 1",
    ] {
        assert!(code.contains(exported), "{exported}\n{code}");
    }
}

#[test]
#[serial]
fn namespace_aliases_read_members() {
    let code = namespace_code(&format!(
        "{NS}const D = Devup\nexport const a = <D.Box bg=\"red\" />\nexport const b = D.css({{ color: 'blue' }})\n"
    ));
    assert!(code.contains("<div className="), "{code}");
    assert!(!code.contains("css("), "{code}");
}

#[test]
#[serial]
fn aliases_in_functions_read_the_package_where_used() {
    let code = namespace_code(
        "import { css, Box } from '@devup-ui/react'\nconst top = css\nexport function f() {\n  const inner = css\n  const again = inner\n  const B = Box\n  const chained = top\n  return [inner({ color: 'red' }), again({ color: 'blue' }), chained({ color: 'green' }), <B bg=\"red\" />]\n}\nexport const g = (css) => { const x = css; return x }\n",
    );
    assert!(!code.contains("inner"), "{code}");
    assert!(!code.contains("again"), "{code}");
    assert!(code.contains("export const g = (css) => {"), "{code}");
    assert!(code.contains("const x = css"), "{code}");
}

#[test]
#[serial]
fn aliases_of_a_barrels_import_in_functions_compile() {
    let project = Project::new(
        "alias-nested",
        &[
            ("ui.ts", "export { css } from '@devup-ui/react'\n"),
            (
                "app.tsx",
                "import { css } from './ui'\nexport function f() {\n  const inner = css\n  return inner({ color: 'red' })\n}\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(!code.contains("inner"), "{code}");
}

#[test]
#[serial]
fn style_constants_read_in_functions_before_their_declaration_compile() {
    let code = namespace_code(
        "import { css, keyframes, Box } from '@devup-ui/react'\nexport function f() {\n  return css({ animation: `${k} 1s` })\n}\nexport const g = () => <Box animationName={k} className={c} />\nexport const h = () => later\nconst k = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })\nconst c = css({ color: 'red' })\nconst later = 1\n",
    );
    assert!(!code.contains("keyframes("), "{code}");
    assert!(!code.contains("${k}"), "{code}");
}

#[test]
#[serial]
fn style_constants_read_before_they_run_are_located_errors() {
    let error = namespace_error(
        "import { css, keyframes } from '@devup-ui/react'\nexport const a = css({ animation: `${k} 1s` })\nconst k = keyframes({ from: { opacity: 0 } })\n",
    );
    assert!(
        error.contains("test.tsx:2:") && error.contains("move that declaration above"),
        "{error}"
    );
}

#[test]
#[serial]
fn style_constants_computed_from_other_bindings_stay_where_they_are() {
    let error = namespace_error(
        "import { css, keyframes } from '@devup-ui/react'\nexport function f() { return css({ animation: `${k} 1s` }) }\nconst frames = { from: { opacity: 0 } }\nconst k = keyframes(frames)\n",
    );
    assert!(error.contains("test.tsx:2:"), "{error}");
}

#[test]
#[serial]
fn aliases_that_are_not_package_exports_stay_declarations() {
    let project = Project::new(
        "alias-other",
        &[
            (
                "ui.tsx",
                "import { Box } from '@devup-ui/react'\nimport * as Devup from '@devup-ui/react'\nexport const N1 = Devup.css.name\nexport const N2 = Box.displayName\nexport const N3 = Devup.useTheme\nexport const N4 = Math.max\nexport const N5 = other.value\n",
            ),
            (
                "app.tsx",
                "import { N1, N2, N3, N4, N5 } from './ui'\nexport const a = [N1, N2, N3, N4, N5]\n",
            ),
        ],
    );
    let code = project.code("app.tsx");
    assert!(code.contains("N1"), "{code}");
    assert!(project.error("ui.tsx").contains("ui.tsx:3:"));
}
