import assert from 'node:assert/strict'
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'

import { openAdapter } from './availability-adapters.mjs'

const [worktree, adapter, order, root, receipt] = process.argv.slice(2)
assert.ok(worktree && root && receipt && ['vite', 'next'].includes(adapter))
assert.ok(['AB', 'BA'].includes(order))
const sources = {
  'src/shared-red.tsx':
    'import { Box } from "@devup-ui/react"; export const Red = () => <Box color="red" />;',
  'src/shared-blue.tsx':
    'import { Box } from "@devup-ui/react"; export const Blue = () => <Box bg="blue" />;',
  'src/app/a/page.tsx':
    'import { Box } from "@devup-ui/react"; import { Red } from "../../shared-red"; import { Blue } from "../../shared-blue"; export default function A() { return <><Red /><Blue /><Box opacity={0.25} /></>; }',
  'src/app/b/page.tsx':
    'import { Box } from "@devup-ui/react"; import { Red } from "../../shared-red"; import { Blue } from "../../shared-blue"; export default function B() { return <><Red /><Blue /><Box opacity={0.75} /></>; }',
}
const files = Object.keys(sources)
const cssDir = join(root, 'df/devup-ui')
rmSync(join(root, 'df'), { recursive: true, force: true })
for (const [file, code] of Object.entries(sources)) {
  mkdirSync(dirname(join(root, file)), { recursive: true })
  writeFileSync(join(root, file), code)
}
mkdirSync(cssDir, { recursive: true })
writeFileSync(join(root, 'devup.json'), '{}')
writeFileSync(join(root, 'tsconfig.json'), '{}')
writeFileSync(
  join(root, 'package.json'),
  '{"name":"availability-fixture","private":true}',
)
process.chdir(root)
delete process.env.DEVUP_HOIST_V
delete process.env.DEVUP_UI_PROFILE
const require = createRequire(
  join(
    worktree,
    'packages',
    `${adapter === 'next' ? 'next' : 'vite'}-plugin`,
    'package.json',
  ),
)
const wasm = require('@devup-ui/wasm')
const artifacts = { js: {}, css: {}, emitted: {} }
const diagnostic = { adapter, order, root, disk: {} }
let seam
try {
  seam = await openAdapter({ adapter, worktree, root, files })
  const canonical = JSON.parse(wasm.exportCanonicalMap())
  const id = (file) =>
    adapter === 'vite' ? join(root, file).replaceAll('\\', '/') : file
  const buckets = files
    .slice(0, 2)
    .map((file) => canonical[id(file)] ?? id(file))
  assert.notEqual(
    buckets[0],
    buckets[1],
    'two physical suppliers must retain distinct buckets',
  )
  assert.ok(buckets.every((bucket) => bucket !== '@global'))
  diagnostic.buckets = buckets
  for (const file of order === 'AB' ? files : [...files].reverse()) {
    artifacts.js[file] = await seam.transform(file, sources[file])
  }
  artifacts.js = Object.fromEntries(
    files.map((file) => [file, artifacts.js[file]]),
  )
  const imports = Object.fromEntries(
    files.map((file) => [
      file,
      [
        ...artifacts.js[file].code.matchAll(
          /["']([^"']*devup-ui(?:-\d+)?\.css(?:\?fileNum=\d+)?)["']/g,
        ),
      ].map((match) => match[1].split('/').pop()),
    ]),
  )
  const names = [
    ...new Set(['devup-ui.css', ...Object.values(imports).flat()]),
  ].sort()
  for (const name of names) artifacts.css[name] = await seam.loadCss(name)
  artifacts.emitted = await seam.finish(names)
  const rules = (text) =>
    [...text.matchAll(/\.([^\s.{:]+)\{([^{}]+)\}/g)].map((match) => [
      match[1],
      match[2].replace(/;$/, ''),
    ])
  const expected = [
    'color:red',
    'background:blue',
    'opacity:.25',
    'opacity:.75',
  ]
  // Vite hoists by default (atomHoist 2): both suppliers are reached by two
  // pages, so their atoms live in the shared base under global short counters
  // while page-only atoms stay private. Next Turbopack suppresses hoisting.
  const hoisted = adapter === 'vite'
  const baseRules = rules(artifacts.css['devup-ui.css'])
  if (hoisted) {
    assert.deepEqual(
      baseRules.map(([, declaration]) => declaration).sort(),
      expected.slice(0, 2).sort(),
      'base must hold exactly the hoisted supplier atoms',
    )
    assert.equal(
      new Set(baseRules.map(([key]) => key)).size,
      baseRules.length,
      'hoisted supplier atoms must keep distinct global counters',
    )
  } else {
    assert.deepEqual(baseRules, [], 'fallback base must have no atoms')
  }
  for (const [index, file] of files.entries()) {
    const generated = [
      ...artifacts.js[file].code.matchAll(
        /className\s*=\s*(?:\{\s*)?["']([^"']+)["']/g,
      ),
    ].flatMap((match) => match[1].trim().split(/\s+/))
    assert.equal(generated.length, 1, 'real JSX extraction required')
    const [name] = generated
    const global = hoisted && index < 2
    assert.match(
      name,
      global ? /^[a-z0-9]+$/ : /^[a-z0-9]+-[a-z0-9]+$/,
      global
        ? 'short global counters required for hoisted atoms'
        : 'short per-file counters required',
    )
    if (global) {
      assert.deepEqual(
        baseRules.filter(([key]) => key === name),
        [[name, expected[index]]],
        'hoisted atom must live in the shared base',
      )
    }
    assert.ok(
      imports[file].some((sheet) => sheet !== 'devup-ui.css'),
      'own local CSS import required',
    )
    const loaded = [...new Set(imports[file])].flatMap((sheet) =>
      rules(artifacts.css[sheet]),
    )
    assert.deepEqual(
      loaded.filter(([key]) => key === name),
      [[name, expected[index]]],
      'each class resolves exactly once in its own loaded CSS',
    )
    assert.deepEqual(
      Object.values(artifacts.css)
        .flatMap(rules)
        .filter(([key]) => key === name),
      [[name, expected[index]]],
      'no duplicate definitions in other sheets',
    )
  }
  for (const [name, text] of Object.entries(artifacts.emitted)) {
    assert.equal(
      text,
      artifacts.css[name],
      'emitted bytes must equal actual loaded CSS',
    )
  }
} finally {
  if (seam) await seam.close()
  for (const name of readdirSync(cssDir)
    .filter((name) => name.endsWith('.css'))
    .sort()) {
    diagnostic.disk[name] = readFileSync(join(cssDir, name), 'utf8')
  }
  // Next disk comments are watch invalidation signals, not CSS-loader output.
  writeFileSync(receipt, JSON.stringify(artifacts))
  writeFileSync(`${receipt}.diagnostic.json`, JSON.stringify(diagnostic))
}
