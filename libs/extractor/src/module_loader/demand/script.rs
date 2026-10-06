use super::super::{Definition, ModuleLoader, ModuleScript, Script};

pub(in crate::module_loader) fn build(loader: &ModuleLoader<'_>, entry: &ModuleScript) -> Script {
    let mut script = Script::default();
    if !entry.commonjs {
        script.generated("\"use strict\";\n");
    }
    for definition in &loader.definitions {
        if let Definition::Generated(text) = definition {
            script.generated(text);
        }
    }
    if let Some((_, name)) = &loader.entry_module {
        let getters = entry
            .exports
            .iter()
            .map(|(exported, local)| {
                format!("{exported:?}:{{get(){{return {local};}},enumerable:true}}")
            })
            .collect::<Vec<_>>()
            .join(",");
        script.generated(&format!("Object.defineProperties({name},{{{getters}}});\n"));
    }
    for definition in &loader.definitions {
        if let Definition::Module {
            head,
            body,
            tail,
            origin,
            ..
        } = definition
        {
            script.generated(head);
            script.body(body, origin);
            script.generated(tail);
        }
    }
    for definition in &loader.definitions {
        if let Definition::Module {
            resume: Some(resume),
            ..
        } = definition
        {
            script.generated(resume);
        }
    }
    if let Some((_, name)) = &loader.entry_module {
        script.generated(&format!("{name}$.start();\n"));
    }
    script.body(&entry.body, &entry.origin);
    script
}
