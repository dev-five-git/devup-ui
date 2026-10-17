import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

const { createModuleResolver } = createRequire(import.meta.url)(
  '../../dist/index.cjs',
)
const root = realpathSync(mkdtempSync(join(tmpdir(), 'devup-tsconfig-native-')))
const file = (path, source) => {
  const target = join(root, path)
  mkdirSync(dirname(target), { recursive: true })
  writeFileSync(target, source)
  return target
}
try {
  // Given both configs and both module targets exist before the first setup.
  file('tsconfig.json', '{"extends":"preset"}')
  const manifest = file(
    'node_modules/preset/package.json',
    '{"exports":"./red.json"}',
  )
  for (const color of ['red', 'blue']) {
    file(
      `node_modules/preset/${color}.json`,
      JSON.stringify({
        compilerOptions: { paths: { color: [`${color}.ts`] } },
      }),
    )
    file(`node_modules/preset/${color}.ts`, `export const color='${color}'`)
  }
  const pid = process.pid
  const first = createModuleResolver({ cwd: root })
  assert.equal(first('color', 'main.ts')?.code, "export const color='red'")
  const fs = createRequire(import.meta.url)('node:fs')
  const originalRead = fs.readFileSync
  const reads = []
  fs.readFileSync = (...args) => {
    reads.push(String(args[0]))
    return originalRead(...args)
  }
  try {
    for (let i = 0; i < 3; i++) first('color', 'main.ts')
    assert.deepEqual(
      reads,
      Array(3).fill(join(root, 'node_modules/preset/red.ts')),
    )
  } finally {
    fs.readFileSync = originalRead
  }
  // When only exports changes and setup is reconstructed in this same native process.
  writeFileSync(manifest, '{"exports":"./blue.json"}')
  const second = createModuleResolver({ cwd: root })
  // Then the new resolver selects blue while the old setup remains frozen.
  assert.equal(second('color', 'main.ts')?.code, "export const color='blue'")
  assert.equal(first('color', 'main.ts')?.code, "export const color='red'")
  assert.equal(process.pid, pid)
  console.info(
    JSON.stringify({
      runtime: process.release.name,
      version: process.version,
      pid,
      before: 'red',
      after: 'blue',
      repeatedReads: reads,
    }),
  )
} finally {
  rmSync(root, { recursive: true, force: true })
}
