use super::*;

#[test]
fn ignored_loader_when_imported_or_reexported_has_empty_namespace() -> Result<(), String> {
    let resolver = |specifier: &str, _: &str| {
        match specifier {
        "empty" => Some(ModuleResolution::Ignored),
        "barrel" => Some(ModuleResolution::Resolved(crate::ResolvedModule {
            path: "/src/barrel.js".to_string(),
            code: "export {default as def, missing as value, __exports__ as hidden} from 'empty'; export * as ns from 'empty'; export * from 'empty';".to_string(),
            source_type: None,
        })),
        "cjs" => Some(ModuleResolution::Resolved(crate::ResolvedModule {
            path: "/src/cjs.js".to_string(),
            code: "const first = require('empty'); const second = require('empty'); const {missing} = require('empty'); module.exports = {ok: first === second && Object.keys(first).length === 0 && first.missing === undefined && missing === undefined};".to_string(),
            source_type: None,
        })),
        _ => None,
    }
    };
    let option = ExtractOption::default();
    let mut loader = ModuleLoader::new(Some(&resolver), &option);
    let script = module_script(
        "import def, {missing, __exports__ as hidden} from 'empty'; import * as ns from 'empty'; import * as barrel from 'barrel'; import cjs from 'cjs'; const ok = missing === undefined && hidden === undefined && Object.keys(def).length === 0 && Object.keys(ns).length === 0 && barrel.value === undefined && barrel.hidden === undefined && Object.keys(barrel.ns).length === 0 && Object.keys(barrel.def).length === 0 && Object.keys(barrel).length === 4 && cjs.ok; ok;",
        "/src/entry.js",
        &mut loader,
        true,
    )?;
    let value = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&format!(
            "{}{}",
            loader.prelude(),
            script.body
        )))
        .map_err(|error| error.to_string())?;
    assert_eq!(value.as_boolean(), Some(true));
    assert_eq!(
        loader.dependencies,
        BTreeSet::from(["/src/barrel.js".to_string(), "/src/cjs.js".to_string()])
    );
    assert_eq!(loader.kept_imports, Vec::<String>::new());
    Ok(())
}

#[test]
fn ignored_named_when_key_is_inherited_is_undefined() -> Result<(), String> {
    let resolver = |_: &str, _: &str| Some(ModuleResolution::Ignored);
    let option = ExtractOption::default();
    for key in ["constructor", "toString", "__proto__", "hasOwnProperty"] {
        let mut loader = ModuleLoader::new(Some(&resolver), &option);
        let script = module_script(
            &format!("import {{ {key} as value }} from 'empty'; typeof value === 'undefined';"),
            "/src/entry.js",
            &mut loader,
            true,
        )?;
        let value = boa_engine::Context::default()
            .eval(boa_engine::Source::from_bytes(&format!(
                "{}{}",
                loader.prelude(),
                script.body
            )))
            .map_err(|error| error.to_string())?;
        assert_eq!(value.as_boolean(), Some(true), "{key}");
        assert_eq!(loader.dependencies, BTreeSet::new());
    }
    Ok(())
}
