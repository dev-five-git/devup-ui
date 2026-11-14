import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  closeSync,
  mkdirSync,
  openSync,
  readdirSync,
  readFileSync,
  realpathSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { compare } from './comparison.mjs'
import { corpus } from './corpus.mjs'

assert.ok(
  process.argv.length === 5 || process.argv.length === 6,
  'Usage: node e2e/binding/identity.mjs <wasm-absolute-entry> <native-absolute-entry> <fresh-evidence-directory> [runtime-executable]',
)
const [wasm, native, evidence, runtime = process.execPath] =
  process.argv.slice(2)
for (const path of [wasm, native, evidence])
  assert.ok(isAbsolute(path), 'Absolute paths required')
mkdirSync(evidence)
const output = realpathSync(evidence),
  here = dirname(fileURLToPath(import.meta.url)),
  root = resolve(here, '../..')
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex')
const save = (path, value) =>
  writeFileSync(join(output, path), JSON.stringify(value, null, 2) + '\n', {
    flag: 'wx',
  })
function sources(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((item) => {
    const path = join(directory, item.name)
    return item.isDirectory() &&
      !['target', 'node_modules', '.git', 'pkg'].includes(item.name)
      ? sources(path)
      : item.isFile() && /\.(rs|toml|lock|mjs|cjs)$/.test(item.name)
        ? [
            {
              path,
              bytes: statSync(path).size,
              sha256: sha(readFileSync(path)),
            },
          ]
        : []
  })
}
const provenance = {
  runner: {
    execPath: process.execPath,
    version: process.version,
    versions: process.versions,
    platform: process.platform,
    arch: process.arch,
  },
  runtime,
  entries: [wasm, native].map((path) => ({
    path,
    realpath: realpathSync(path),
    sha256: sha(readFileSync(path)),
  })),
  sources: [
    ...sources(join(root, 'libs')),
    ...sources(join(root, 'bindings')),
    ...sources(here),
    ...['Cargo.toml', 'Cargo.lock'].map((name) => {
      const path = join(root, name)
      return { path, sha256: sha(readFileSync(path)) }
    }),
  ],
}
save('provenance.json', provenance)
const runs = [],
  problems = []
for (const [engine, entry] of [
  ['wasm', wasm],
  ['native', native],
])
  for (let repeat = 1; repeat <= 3; repeat++) {
    const name = `${engine}-${repeat}`,
      directory = join(output, name)
    mkdirSync(directory)
    const stdout = openSync(join(directory, 'stdout.log'), 'wx'),
      stderr = openSync(join(directory, 'stderr.log'), 'wx')
    let result
    try {
      result = spawnSync(runtime, [join(here, 'host.mjs'), entry, directory], {
        cwd: directory,
        env: { ...process.env, NODE_OPTIONS: '', BUN_OPTIONS: '' },
        stdio: ['ignore', stdout, stderr],
        timeout: 120000,
        killSignal: 'SIGKILL',
      })
    } finally {
      closeSync(stdout)
      closeSync(stderr)
    }
    const run = {
      name,
      engine,
      repeat,
      command: [runtime, join(here, 'host.mjs'), entry, directory],
      status: result.status,
      signal: result.signal,
      error: result.error
        ? {
            name: result.error.name,
            message: result.error.message,
            code: result.error.code,
          }
        : null,
    }
    runs.push(run)
    if (result.error || result.signal !== null || result.status !== 0)
      problems.push({ run: name, reason: 'natural zero exit required' })
    try {
      const records = readFileSync(join(directory, 'records.jsonl'))
      run.recordBytes = records.length
      run.recordSha256 = sha(records)
      const parsed = records
        .toString('utf8')
        .trimEnd()
        .split('\n')
        .map((line) => JSON.parse(line))
      assert.deepEqual(
        parsed
          .filter((record) => record.kind === 'complete')
          .map((record) => record.scenario),
        corpus.map((test) => test.name),
      )
      assert.ok(parsed.every((record, i) => record.sequence === i))
      const host = JSON.parse(
        readFileSync(join(directory, 'provenance.json'), 'utf8'),
      )
      assert.equal(host.loaded, realpathSync(entry))
      const abi = JSON.parse(readFileSync(join(directory, 'abi.json'), 'utf8'))
      assert.equal(abi.result.tag, 'return')
    } catch (error) {
      problems.push({ run: name, reason: error.message })
    }
    run.artifacts = readdirSync(directory).map((path) => ({
      path,
      bytes: statSync(join(directory, path)).size,
      sha256: sha(readFileSync(join(directory, path))),
    }))
    save(`${name}.json`, run)
  }
const comparisons = compare(output, problems)
save('comparison.json', {
  verdict: problems.length ? 'FAIL' : 'PASS',
  scenarios: corpus.map((test) => test.name),
  runs,
  comparisons,
  problems,
})
console.info(`${problems.length ? 'FAIL' : 'PASS'}: ${output}`)
if (problems.length) process.exitCode = 1
