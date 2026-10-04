import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import {
  buildStaticImportGraph,
  createModuleResolver,
  listSourceFiles,
} from '../import-graph'
import { collectNumberedFiles } from '../numbering'

let root: string
beforeEach(() => {
  const parent = join(
    tmpdir(),
    'opencode',
    'workers',
    'w20-plugins-core',
    'shared-api',
  )
  mkdirSync(parent, { recursive: true })
  root = realpathSync.native(mkdtempSync(join(parent, 'source-')))
})
afterEach(() => rmSync(root, { recursive: true, force: true }))
function file(path: string, source = 'export {}'): string {
  const target = join(root, path)
  mkdirSync(dirname(target), { recursive: true })
  writeFileSync(target, source)
  return target
}

it('keeps Markdown off by default and selects exact extensions in scanner and numbering', () => {
  const js = file('src/main.ts')
  const md = file('src/page.md')
  file('src/page.mdx')
  const custom = file('src/page.mdown')
  file('src/generated/skip.md')
  expect(listSourceFiles(join(root, 'src'))).toEqual([js])
  expect(
    listSourceFiles(join(root, 'src'), ['generated'], { includeMdx: ['.md'] }),
  ).toEqual([js, md])
  expect(
    collectNumberedFiles({
      roots: [join(root, 'src')],
      exclude: ['generated'],
      includeMdx: ['.md'],
      toId: (path) => path,
    }),
  ).toEqual([js, md])
  expect(
    listSourceFiles(join(root, 'src'), [], { includeMdx: ['.mdown'] }),
  ).toEqual([js, custom])
  expect(
    collectNumberedFiles({
      roots: [join(root, 'src')],
      includeMdx: ['.mdown'],
      toId: (path) => path,
    }),
  ).toEqual([js, custom])
})

it('returns a Promise for a synchronous preparer and follows a project-root provider once', async () => {
  const entry = file('src/page.mdx', '# Raw markdown')
  const provider = file('mdx-components.js', "import './value.generated'")
  const leaf = file('value.generated.mjs')
  const visits: string[] = []
  const graphPromise = buildStaticImportGraph('src', undefined, {
    cwd: root,
    includeMdx: true,
    alias: { provider },
    prepareSource: (filename: string) => {
      visits.push(filename)
      return filename === entry ? "import 'provider'" : undefined
    },
  })
  expect(graphPromise).toBeInstanceOf(Promise)
  const graph = await graphPromise
  expect(graph.files).toEqual([entry, provider, leaf].sort())
  expect(visits.sort()).toEqual(graph.files)
})

it('does not discover import examples inside strings, regex or JSX text', () => {
  const entry = file(
    'src/main.tsx',
    [
      'const example = "import \'./fake\'";',
      'const poison = "import \'fake-package\'";',
      String.raw`const pattern = /"import '\.\/fake'"/;`,
      "const template = `before ${`nested import './fake'`} after`;",
      'const element = <div>import \'./fake\' {import("./real")}</div>;',
      "import './real'",
    ].join('\n'),
  )
  const real = file('src/real.ts')
  file('src/fake.ts')
  file('node_modules/fake-package/package.json', '{')
  const graph = buildStaticImportGraph('src', undefined, { cwd: root })
  expect(graph.staticImports.get(entry)).toEqual(new Set([real]))
  expect(graph.dynamicImports.get(entry)).toEqual(new Set([real]))
})

it('does not leak template content after regex braces or nested escaped templates', () => {
  const entry = file(
    'src/main.ts',
    [
      'const example = tag`prefix ${ /}/.test("x") ? `nested import "./fake"` : "" } tail import "./fake"`;',
      "import './real'",
    ].join('\n'),
  )
  const real = file('src/real.ts')
  file('src/fake.ts')
  expect(
    buildStaticImportGraph('src', undefined, { cwd: root }).staticImports.get(
      entry,
    ),
  ).toEqual(new Set([real]))
})

it('masks JSX text after arrow and conditional expression boundaries', () => {
  const entry = file(
    'src/main.tsx',
    [
      'const View = () => <div>import "./fake" {import("./real")}</div>;',
      'const branch = flag ? <div>import "./fake"</div> : <span/>;',
      'const optional = flag && <div>import "./fake"</div>;',
      "import './real'",
    ].join('\n'),
  )
  const real = file('src/real.ts')
  file('src/fake.ts')
  const graph = buildStaticImportGraph('src', undefined, { cwd: root })
  expect(graph.staticImports.get(entry)).toEqual(new Set([real]))
  expect(graph.dynamicImports.get(entry)).toEqual(new Set([real]))
})

it('rewrites aliases before tsconfig and never falls through a matched missing target', () => {
  const entry = file('src/main.ts')
  const target = file('actual/index.js')
  file(
    'tsconfig.json',
    JSON.stringify({ compilerOptions: { paths: { name: ['./fallback.js'] } } }),
  )
  file('fallback.js')
  const resolver = createModuleResolver({
    cwd: root,
    alias: { name$: target, missing: join(root, 'absent') },
  })
  expect(resolver('name', entry)?.path).toBe(target)
  file('node_modules/missing/package.json', '{"main":"index.js"}')
  file('node_modules/missing/index.js')
  expect(resolver('missing', entry)).toBeUndefined()
})

it('does not widen local graph roots merely because an empty alias table exists', () => {
  const entry = file('src/main.ts', "import '../outside'")
  file('outside.js')
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    alias: {},
  })
  expect(graph.files).toEqual([entry])
})
