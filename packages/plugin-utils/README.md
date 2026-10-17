# @devup-ui/plugin-utils

Shared build-time utilities for Devup UI plugins. These utilities do not execute
modules or add styling JavaScript to application output.

## Source selection and exclusions

`listSourceFiles`, `buildStaticImportGraph`, and `collectNumberedFiles` all default
to JavaScript/TypeScript only (`.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`,
`.cjs`). Markdown is **off by default everywhere**.

Their `includeMdx` option has type `boolean | readonly string[]`:

- Omitted, `false`, or `[]`: no extra extensions.
- `true`: exactly `.mdx`, not `.md`.
- A list: exactly the supplied literal extensions, such as `['.md', '.mdx']`
  or `['.mdown']`. Use normalized extension strings with their leading dot;
  matching is case-insensitive. No extension patterns or regular expressions.

```ts
import {
  buildStaticImportGraph,
  collectNumberedFiles,
  listSourceFiles,
} from '@devup-ui/plugin-utils'

const js = listSourceFiles('/project/src', ['generated'])
const docs = listSourceFiles('/project/src', ['generated'], {
  includeMdx: ['.md', '.mdx', '.mdown'],
})
const numbered = collectNumberedFiles({
  roots: ['/project/src'],
  includeMdx: ['.mdown'],
  exclude: ['/project/build/next', 'generated'],
  needles: ['@devup-ui/react'],
})
```

The two-argument scanner call retains its directory-exclusion argument; use the
third options argument to opt into Markdown. All three APIs use one exclusion
matcher: a bare directory name matches at any depth; an absolute directory path
matches only that directory and its descendants, not same-basename folders
elsewhere. Windows matching normalizes separators, drive letters and path case;
POSIX matching preserves case. Excluded roots and imported targets are skipped
before reading source, preparation or numbering needle checks. `node_modules`
and test/spec files remain absent from directory enumeration.

Opt-in extensions also participate in extensionless graph resolution and route
recognition. Explicit existing file paths still resolve with
`createModuleResolver` without opting into their extension, but Markdown must be
prepared before its source can be returned. Numbering forwards
selection and exclusions to the scanner rather than reading then filtering.
The public module resolver retains its legacy extensionless `.mdx` probing;
the graph's path-only resolver probes extra extensions only when selected.

## Synchronous and prepared-source graphs

Without a `prepareSource` field, `buildStaticImportGraph` remains synchronous and
returns `StaticImportGraph`. Existing synchronous callers do not receive a
`StaticImportGraph | Promise<StaticImportGraph>` union.

```ts
const graph = buildStaticImportGraph('src', undefined, { cwd: '/project' })

const prepared = await buildStaticImportGraph('src', undefined, {
  cwd: '/project',
  includeMdx: ['.mdx', '.mdown'],
  alias: { 'mdx-provider$': '/project/mdx-components.js' },
  prepareSource: async (filename) => {
    if (!filename.endsWith('.mdx') && !filename.endsWith('.mdown')) return undefined
    const compiled = await compileProjectMarkdown(filename)
    return {
      code: String(compiled.value),
      map: compiled.map,
      sourceType: 'compiled-mdx',
    }
  },
})
```

Supplying **any** preparer, including a synchronous callback, returns
`Promise<StaticImportGraph>`. The exported `PrepareSource` receives each visited
real absolute filename once, including discovered providers and eligible
included-package dependencies. It returns a string, `{ code: string, map?:
unknown }`, `undefined`, or a Promise of those values. Only `undefined` reads raw
source; `''` is valid prepared code. Relative imports, keys and diagnostics keep
the actual filename. Prepared code bypasses the raw Markdown ESM-block filter,
so compiler-injected provider/remark/rehype imports can be discovered. Configure
the project compiler to interpret custom extensions as MDX where required.

Prepared objects and resolved modules accept `sourceType?: 'compiled-mdx'`.
This explicitly selects JavaScript with JSX, including under `.mdown` or a TS
filename; ordinary filename-derived JS/TS grammar is unchanged when omitted.
The extraction functions receive this value as their optional ninth argument.
`.md` and `.mdx` retain their compiled-MDX extractor defaults; a custom extraction
extension without an explicit type is a located error, never a renamed-file
workaround. Invalid values or throwing type getters fail at the preparation
boundary with the real filename/importer and cause. The field follows prepared
resolver generations into WASM, including imported constants after JSX and
reexports, and caches include the selected type.

The graph is an **import scanner, not a full syntax validator**. The caller's
compiler and bundler own validation of the same compiled output. No parser
dependency is installed by this package. **Every edge always comes from one
dependency-free scanner.** If optional `oxc-parser` is available, it supplies
syntax diagnostics only: its AST and `program` getter never determine edges.
Parser exceptions/diagnostics fail rather than falling back to raw source or
changing edge authority. Compiler failures retain the real filename and
original cause; setup/hook failures reject the Promise. Parser positions are
remapped using `remapMdxError` when a map is supplied; unmapped positions are
explicitly labeled `in compiled output`, not presented as Markdown coordinates.

The lexer keeps real filename language context: comparisons/shifts are not JSX,
TSX generic arrows are not tags, and type-only imports are not runtime edges.
Prepared ordinary TS retains its TS grammar; explicit compiled-MDX output uses
JS/JSX grammar even under a TS filename. The lexer masks ordinary strings, outer
template text, comments, regular
expressions and JSX text/quoted attributes. Template `${...}` expressions,
including nested templates, and JSX brace/attribute expressions are real code:
literal `require('...')` adds static edges and literal `import('...')` adds dynamic
edges there just as in ordinary code. Normal imports, side-effect imports and
export-from declarations add static edges; type-only declarations/specifiers are
elided without dropping value bindings named `type`. Escaped specifiers are
decoded without evaluating source. Nonliteral/computed or concatenated arguments
such as `import(name)`, `require(name)` and `import('./' + name)` add no edge.
The scanner is not a full parser; builders retain fail-closed checks for modules
the bundler actually loads but the static graph misses. Raw opted-in Markdown
scans only ESM blocks, not code fences or prose.

The checked-in differential gate is runnable from the repository root with
`bun test ./packages/plugin-utils/src/__tests__/parser-corpus.test.ts` (and runs
in the root suite). It compares complete graphs for every package `src` root and
`apps/landing/src`, including real landing docs compiled once using the project's
MDX compiler/provider options, with diagnostics disabled and injected. It asserts
one diagnostic invocation per source and zero AST getter reads; independent
targeted fixtures also assert expected literal edges and reject contradictory AST
dependencies. The optional Oxc package is not installed by this gate.

## Synchronous prepared module resolver

`createModuleResolver({ prepareSource })` retains its synchronous callable shape
for `setModuleResolver` and also exposes `resolver.remapError(error)`. The hook
receives the real absolute filename after native/alias resolution and before
`toId`; it returns the same string, `{ code, map? }`, or `undefined` values as the
graph hook. Empty code is valid. Ordinary JS/TS with `undefined` still reads raw
source. The resolver never awaits: **any thenable is rejected immediately**.
Async compilers must finish preparation into a caller-owned cache first, then
pass the same synchronous lookup hook to both graph and resolver.

With or without a hook, `.mdx`, `.md`, and explicit `includeMdx` extensions cannot
reach WASM without prepared JavaScript. Missing preparation, thenables and hook
exceptions fail at the actual importer `:1:1` (the callback has no import span),
name the actual module and preserve the cause. Move imported values into JS/TS
when Markdown compilation is not available; this API does not add MDX support
to plugins that do not compile it.

Callers supplying `prepareSource` **must route every error thrown by
`codeExtract` through `resolver.remapError`**, not just callback errors:

```ts
const prepareSource = (filename: string) => compiledCache.get(filename)
const resolver = createModuleResolver({ cwd, toId, prepareSource })
setModuleResolver(resolver)
try {
  return codeExtract(filename, source, /* existing extraction arguments */)
} catch (error) {
  throw resolver.remapError(error)
}
```

Mappings are resolver-instance-local, keyed by the exact id returned to WASM.
Each resolved generation replaces its prior code/map; ordinary raw JS/TS removes
its stale map. Keep the resolver for its build/compiler context, replace cache
entries when recompiling, and discard it when that context ends. Vite and Bun
create build-context resolvers; Webpack creates compiler/loader-context resolvers.
Bun supplies this hook for its own compiled MDX cache and wraps extraction
errors; Next's prepared-source integration on #752 also owns its preparation
and wrapping. Vite, Webpack and Rsbuild do not gain a separate project MDX
preparation cache from this shared API.

Every exact prepared-id location in an error is remapped through `remapMdxError`;
other file locations are untouched. Missing/uncovered mappings explicitly say
`in compiled output` of the real file. Returned errors retain the original as
`cause`; passing the returned error to the same resolver again is idempotent.
The resolver cannot recover imported-file coordinates absent from the extractor
error: that mapping boundary belongs to #757. With #755 alone, the real imported
invalid-value case reports the importing TSX source position, which stays
untouched. Contract tests use synthetic extractor-format diagnostics and real
compiler maps; the imported-module end-to-end mapping check runs after #755 and
#757 are both on main, without merging either automatically.

## Resolver aliases

Both `createModuleResolver` and graph options accept an optional readonly
`alias: ModuleAliasOptions`. The existing `ModuleAliases` map type remains
`Readonly<Record<string, string | false | readonly (string | false)[]>>`.
The option also accepts native `false` (disable aliases) or ordered descriptors:

```ts
const alias = [
  { name: 'provider', alias: ['first-provider', false], onlyModule: true },
  { name: 'provider$', alias: false, onlyModule: false },
] as const
```

Descriptors preserve declaration order and duplicate names; a literal `$` in
their name remains a prefix when `onlyModule` is false. In the map form,
`provider$` instead means the exact name `provider`.
Targets are strings, `false`, or ordered string/false candidate arrays:
absolute file/directory paths or package requests. They run before tsconfig
paths, baseUrl and package resolution.

- `name$` matches only `name`; `name` matches `name` and `name/subpath`, not
  `name-other`.
- Declaration order matters: the first **rewriting** match wins. Put a specific
  key before an overlapping broad key.
- Each candidate undergoes full native resolution in order; the first resolving
  candidate wins. A failed matched key never falls through to the original request
  or a later key. All unresolved rewriting candidates produce an
  importer-located error naming the key and every rewritten candidate tried.
  Later descriptors with the same name do not bypass that failure. Empty arrays
  and selfguard-only entries perform no rewrite, so they continue to the next
  descriptor/key and then the raw request. Unaliased unresolved requests still
  return `undefined`.
- Finite chains resolve; self-aliases are skipped; cycles produce an
  importer-located failure.
- A reached `false` immediately resolves to `{ ignored: true }`, including when
  reached through another string alias. String candidates re-enter the alias map;
  a request equal to or below its target is not aliased again. Earlier resolving
  candidates win, and newly created earlier files are visible on the next call.
- Package exports and active conditions are applied after rewriting. Graph
  package eligibility uses the actual target package, not the alias name.
- Filesystem directory requests read their own `package.json` and try the shared
  `module`, `main`, then index order after file/extension probes. These absolute
  or relative directory paths follow native main-field resolution, not a bare
  package's exports restriction. Recursive entries honor exclusions before IO;
  malformed manifests and directory cycles are located errors. Installed Next's
  absolute `@swc/helpers/_` prefix alias is covered with the real helper package.
- Export/active-target failures, configuration/I/O errors and cycles are fatal,
  not ordinary misses that advance to the next candidate. Excluded candidates
  are skipped before I/O; an entirely excluded list stays excluded, not external.
- Project-local aliased providers outside configured source roots can be
  followed; excluded directories and excluded external packages remain excluded.

Ignored results have no `path`, `code` or `sourceType`. `ModuleResolution` is
`ResolvedModule | IgnoredModule`; narrow the ignored result before reading file
fields. Neither the graph nor the evaluator scans, prepares, watches or numbers
a manufactured ignored file. Physically enumerated real source files retain
their normal numbering. Evaluated ignored imports use an empty CommonJS export
object: missing named values are `undefined`, the default follows existing
CommonJS interop, and namespace/require values are empty objects.

This subset is tested against installed enhanced-resolve 5.25.1 with real files.
Its `AliasUtils` supports reached false candidates, but its public
`ResolverFactory` currently rejects mixed string/false arrays before that handler;
Webpack therefore stops with its own construction error for those arrays.
Rspack's native map-or-false format accepts mixed false arrays and has the same
terminal-rewrite/empty-array behavior. Webpack passes its native map or ordered
descriptors unchanged through graph, setup and loader resolver/cache; Rsbuild
passes the actual Rspack map-or-false form. Unsupported wildcard alias names fail
with an importer-located configuration error naming the key instead of being
silently dropped.
