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
`createModuleResolver` without opting into their extension. Numbering forwards
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
    return { code: String(compiled.value), map: compiled.map }
  },
})
```

Supplying **any** preparer, including a synchronous callback, returns
`Promise<StaticImportGraph>`. The exported `PrepareSource` receives each visited
real absolute filename once, including discovered providers and eligible
included-package dependencies. It returns a string, `{ code: string, map?:
unknown }`, `undefined`, or a Promise of those values. Only `undefined` reads raw
source; `''` is valid prepared code. Relative imports, keys and diagnostics keep
the actual filename. Prepared JS/JSX bypasses the raw Markdown ESM-block filter,
so compiler-injected provider/remark/rehype imports can be discovered. Configure
the project compiler to interpret custom extensions as MDX where required.

The graph is an **import scanner, not a full syntax validator**. The caller's
compiler and bundler own validation of the same compiled output. No parser
dependency is installed by this package. If optional `oxc-parser` is available,
its prepared-source exceptions/diagnostics fail rather than falling back to raw
source or lexical scanning. Compiler failures retain the real filename and
original cause; setup/hook failures reject the Promise. Parser positions are
remapped using `remapMdxError` when a map is supplied; unmapped positions are
explicitly labeled `in compiled output`, not presented as Markdown coordinates.

The dependency-free lexer masks ordinary strings, template contents, comments,
regular expressions and JSX text, while preserving import literal arguments and
JSX expression imports. It retains existing type-only elision. It is not a
general JavaScript parser: computed imports, CommonJS `require` discovery and
template-interpolation import discovery are not a new guarantee. The legacy
optional AST/fallback difference for template-interpolation imports is unchanged.
Raw opted-in Markdown scans only ESM blocks, not code fences or prose.

## Resolver aliases

Both `createModuleResolver` and graph options accept an optional readonly
`alias: Record<string, string>`. Supported aliases are string targets only:
absolute file/directory paths or package requests. They run before tsconfig
paths, baseUrl and package resolution.

- `name$` matches only `name`; `name` matches `name` and `name/subpath`, not
  `name-other`.
- Declaration order matters: the first **rewriting** match wins. Put a specific
  key before an overlapping broad key.
- A failed rewritten target never falls through to the original request or a
  later alias. Ordinary unresolved targets return `undefined`.
- Finite chains resolve; self-aliases are skipped; cycles produce an
  importer-located failure.
- Package exports and active conditions are applied after rewriting. Graph
  package eligibility uses the actual target package, not the alias name.
- Project-local aliased providers outside configured source roots can be
  followed; excluded directories and excluded external packages remain excluded.

This subset is tested against installed enhanced-resolve 5.25.1 with real files.
Alias `false`, array targets and wildcard keys are not supported by this API.
