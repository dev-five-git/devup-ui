import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  closeSync,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from 'node:fs'
import { dirname, isAbsolute, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { hash } from './observation.mjs'
import { cases, positions } from './validation-cases.mjs'

assert.ok(
  process.argv.length === 5 || process.argv.length === 6,
  'Usage: node e2e/binding/validation.mjs <wasm-absolute-entry> <native-absolute-entry> <fresh-evidence-directory> [runtime-executable]',
)
const [wasm, native, evidence, runtime = process.execPath] =
  process.argv.slice(2)
for (const path of [wasm, native, evidence])
  assert.ok(isAbsolute(path), 'Absolute paths required')
mkdirSync(evidence)
const output = realpathSync(evidence),
  here = dirname(fileURLToPath(import.meta.url))
const save = (name, value) =>
  writeFileSync(join(output, name), JSON.stringify(value, null, 2) + '\n', {
    flag: 'wx',
  })
save('provenance.json', {
  runtime,
  runner: {
    execPath: process.execPath,
    version: process.version,
    versions: process.versions,
  },
  entries: [wasm, native].map((path) => ({
    path,
    loaded: realpathSync(path),
    sha256: hash(path),
  })),
  sources: [
    'validation.mjs',
    'validation-host.mjs',
    'validation-cases.mjs',
    'observation.mjs',
    'corpus.mjs',
    'corpus/extraction.mjs',
  ].map((name) => ({ name, sha256: hash(join(here, name)) })),
})
const runs = []
for (const [engine, entry] of [
  ['wasm', wasm],
  ['native', native],
]) {
  for (const test of cases) {
    const name = `${engine}-${test.id}`,
      directory = join(output, name)
    mkdirSync(directory)
    const stdout = openSync(join(directory, 'stdout.log'), 'wx'),
      stderr = openSync(join(directory, 'stderr.log'), 'wx')
    let result
    try {
      result = spawnSync(
        runtime,
        [join(here, 'validation-host.mjs'), entry, directory, test.id],
        {
          cwd: directory,
          env: { ...process.env, NODE_OPTIONS: '', BUN_OPTIONS: '' },
          stdio: ['ignore', stdout, stderr],
          timeout: 120000,
          killSignal: 'SIGKILL',
        },
      )
    } finally {
      closeSync(stdout)
      closeSync(stderr)
    }
    const crash = Boolean(
      result.error || result.signal || result.status === null,
    )
    const missing = !existsSync(join(directory, 'record.json'))
    const complete = existsSync(join(directory, 'complete.json'))
    const failed = crash || missing || result.status !== 0 || !complete
    const run = {
      engine,
      id: test.id,
      status: result.status,
      signal: result.signal,
      error: result.error
        ? {
            name: result.error.name,
            message: result.error.message,
            code: result.error.code,
          }
        : null,
      crash,
      missing,
      failed,
    }
    if (!missing) run.recordSha256 = hash(join(directory, 'record.json'))
    if (complete)
      assert.equal(
        JSON.parse(readFileSync(join(directory, 'complete.json'), 'utf8')).id,
        test.id,
      )
    runs.push(run)
    save(`${name}.json`, run)
  }
}
const summary = {
  cases: cases.length,
  positions: positions.length,
  functions: 18,
  engines: 2,
  runs: runs.length,
  failed: runs.filter((run) => run.failed).length,
  missing: runs.filter((run) => run.missing).length,
  crash: runs.filter((run) => run.crash).length,
}
save('summary.json', { ...summary, runs })
console.info(JSON.stringify(summary))
if (summary.failed) process.exitCode = 1
