use super::*;
use crate::extract_style::compiler_associations::Graph;
use crate::extract_style::compiler_receipts::{PRODUCTIONS, ReceiptWitness};
use crate::extract_style::extract_style_value::ExtractStyleValue;
use serial_test::serial;
#[path = "compiler_surface_tests.rs"]
mod surface_tests;
pub(crate) use surface_tests::names;

pub(crate) use crate::sparse_sites::tests::observe_variable;
pub(crate) fn input<'a>(filename: &'a str, code: &'a str) -> CompilerInput<'a> {
    CompilerInput {
        filename,
        code,
        option: ExtractOption::default(),
        source_map: false,
        resolver: None,
    }
}
pub(crate) fn associated(
    input: CompilerInput<'_>,
) -> (
    CompilerOutputMetadata,
    Vec<ExtractStyleValue>,
    Vec<ReceiptWitness>,
    Graph,
) {
    with_counter_extract(input, |metadata, batch| {
        batch.consume(|view| {
            Ok::<_, ()>((
                metadata,
                view.styles.to_vec(),
                view.witnesses.to_vec(),
                view.associations.clone(),
            ))
        })
    })
    .unwrap_or_else(|error| panic!("{error:?}"))
}
pub(crate) fn failed(input: CompilerInput<'_>) -> Box<dyn Error> {
    match with_counter_extract(input, |_, batch| batch.consume(|_| Ok::<_, ()>(()))) {
        Err(CounterCompileError::Compile(error)) => error,
        Err(CounterCompileError::Consumer(())) | Ok(()) => {
            panic!("terminal compile error required")
        }
    }
}
pub(crate) fn offset(source: &str, token: &str) -> u32 {
    u32::try_from(
        source
            .find(token)
            .unwrap_or_else(|| panic!("authored token")),
    )
    .unwrap_or_else(|_| panic!("Oxc offset"))
}

type Observer = Box<dyn FnMut(bool)>;
thread_local! {
    static OBSERVER: std::cell::RefCell<Option<Observer>> = const { std::cell::RefCell::new(None) };
}
pub(crate) fn observe(retry: bool) {
    let mut observer = OBSERVER.take();
    if let Some(observer) = &mut observer {
        observer(retry);
    }
    OBSERVER.set(observer);
}
pub(crate) struct ObserveScope(std::collections::HashMap<String, String>);
impl ObserveScope {
    pub(crate) fn enter(observer: impl FnMut(bool) + 'static) -> Self {
        OBSERVER.set(Some(Box::new(observer)));
        Self(css::file_map::get_canonical_map())
    }
}
impl Drop for ObserveScope {
    fn drop(&mut self) {
        OBSERVER.set(None);
        css::file_map::set_canonical_map(std::mem::take(&mut self.0));
    }
}

pub(crate) fn reset() {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    PRODUCTIONS.set(0);
}
pub(crate) fn compile(
    filename: &str,
    code: &str,
) -> (
    CompilerOutputMetadata,
    Vec<ExtractStyleValue>,
    Vec<ReceiptWitness>,
) {
    let mut source = input(filename, code);
    source.source_map = true;
    let (metadata, styles, receipts, _) = associated(source);
    (metadata, styles, receipts)
}
#[test]
#[serial]
fn raw_originals_register_before_no_style_returns() {
    reset();
    let raw = "raw/../A.tsx";
    let _ = compile(raw, "const a = 1;");
    let _ = compile("B.tsx", "const b = 2;");
    assert_eq!(
        css::file_map::get_original_ids(),
        std::collections::BTreeMap::from([(raw.into(), 0), ("B.tsx".into(), 1)])
    );
    assert_eq!(PRODUCTIONS.get(), 0);
}
#[test]
#[serial]
fn seeded_raw_original_is_reused_by_real_parser() {
    reset();
    css::file_map::set_original_ids(std::collections::BTreeMap::from([("A.tsx".into(), 7)]));
    let (_, styles, _) = compile(
        "A.tsx",
        "import {Box} from '@devup-ui/react'; const a = <Box color={value}/>;",
    );
    let ExtractStyleValue::Dynamic(style) = &styles[0] else {
        panic!("dynamic")
    };
    assert_eq!(
        style.site().map(|site| &site.file),
        Some(&css::sparse_site::SourceFile::D9(7))
    );
    assert_eq!(css::file_map::get_original_ids()["A.tsx"], 7);
}

#[test]
#[serial]
fn counter_site_terminal_prevents_otherwise_evaluable_retry() {
    reset();
    let source =
        "import {css} from '@devup-ui/react'; const pick=()=> 'red'; const s=css({color:pick()});";
    let at = offset(source, "pick()");
    let visits = std::rc::Rc::new(Cell::new(0));
    let observed = visits.clone();
    let _observer = ObserveScope::enter(move |retry| {
        if !retry {
            observed.set(observed.get() + 1);
            crate::provenance::site_at(at, 0, "first");
            crate::provenance::site_at(at, 0, "second");
        }
    });
    let error = failed(input("terminal.tsx", source));
    assert_eq!(
        visits.get(),
        1,
        "a terminal role collision must precede evaluation"
    );
    assert!(error.is::<projection::ProjectionError>(), "{error}");
}

#[test]
#[serial]
fn module_terminal_survives_parent_early_error_and_string_transport() {
    reset();
    let mut injected = false;
    let _observer = ObserveScope::enter(move |retry| {
        if !retry && !injected {
            injected = true;
            receipts::collect::<()>(
                Err(projection::ProjectionError::Producer(
                    crate::extract_style::CounterProducerError::UnnumberedSite,
                )),
                Some(0),
            );
        }
    });
    let resolver = |specifier: &str, _: &str| {
        Some(if specifier == "./middle.css" {
            crate::ResolvedModule {
            path: "middle.css.ts".into(),
            code: "import {s} from './child.css'; import {style} from '@devup-ui/react'; export const s2=style([s]);".into(),
        }
        } else {
            crate::ResolvedModule {
                path: "child.css.ts".into(),
                code: "import {style} from '@devup-ui/react'; export const s=style({color:'red'});"
                    .into(),
            }
        })
    };
    let mut source = input(
        "parent.css.ts",
        "import {s2} from './middle.css'; import {style} from '@devup-ui/react'; export const root=style([s2]);",
    );
    source.resolver = Some(&resolver);
    let error = failed(source);
    assert!(
        error.is::<projection::ProjectionError>(),
        "typed fatal was erased: {error}"
    );
    assert!(error.to_string().contains("child.css.ts"), "{error}");
}

#[test]
#[serial]
fn successful_no_style_invoke_drains_pending_typed_terminal() {
    reset();
    let resolver = |_: &str, _: &str| {
        receipts::collect::<()>(
            Err(projection::ProjectionError::Producer(
                crate::extract_style::CounterProducerError::UnnumberedSite,
            )),
            None,
        );
        Some(crate::ResolvedModule {
            path: "constant.ts".into(),
            code: "export const v=1;".into(),
        })
    };
    let published = Cell::new(false);
    let mut source = input(
        "empty.css.ts",
        "import {v} from './constant'; import {css} from '@devup-ui/react'; const x=v;",
    );
    source.resolver = Some(&resolver);
    let result = with_counter_extract(source, |_, batch| {
        published.set(true);
        batch.consume(|_| Ok::<_, ()>(()))
    });
    let Err(CounterCompileError::Compile(error)) = result else {
        panic!("pending fatal must not publish")
    };
    assert!(error.is::<projection::ProjectionError>(), "{error}");
    assert!(!published.get());
    assert!(
        error.to_string().contains("position unavailable"),
        "{error}"
    );
}
