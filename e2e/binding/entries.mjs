import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  closeSync,
  mkdirSync,
  openSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from 'node:fs'
import { dirname, isAbsolute, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { hash } from './observation.mjs'

assert.equal(
  process.argv.length,
  7,
  'Usage: entries.mjs <absolute-node> <absolute-bun> <NativeB> <WASM> <fresh-evidence-directory>',
)
const [node, bun, nativeB, wasm, evidence] = process.argv.slice(2)
for (const path of [node, bun, nativeB, wasm, evidence])
  assert.ok(isAbsolute(path), 'All paths must be absolute')
const here = dirname(fileURLToPath(import.meta.url))
mkdirSync(evidence)
const save = (name, value) =>
  writeFileSync(join(evidence, name), JSON.stringify(value, null, 2) + '\n', {
    flag: 'wx',
  })
save('provenance.json', {
  runtimes: [node, bun].map((path) => ({
    path,
    real: realpathSync(path),
    sha256: hash(path),
  })),
  engines: [nativeB, wasm].map((path) => ({
    path,
    real: realpathSync(path),
    sha256: hash(path),
  })),
  sources: [
    'entries.mjs',
    'entries-host.mjs',
    'entries-cases.mjs',
    'entries-errors.mjs',
    'entries-context.mjs',
    'entries-fixtures.mjs',
    'entries-slots.mjs',
    'entries-queues.mjs',
    'observation.mjs',
    'corpus/extraction.mjs',
  ].map((name) => ({ name, sha256: hash(join(here, name)) })),
})
const runs = []
let reference
for (const [host, runtime] of [
  ['node', node],
  ['bun', bun],
]) {
  for (const [engine, entry] of [
    ['NativeB', nativeB],
    ['WASM', wasm],
  ]) {
    for (let repeat = 1; repeat <= 3; repeat++) {
      const id = `${host}-${engine}-${repeat}`,
        directory = join(evidence, id)
      mkdirSync(directory)
      const stdout = openSync(join(directory, 'stdout.log'), 'wx')
      const stderr = openSync(join(directory, 'stderr.log'), 'wx')
      let child
      try {
        child = spawnSync(
          runtime,
          [join(here, 'entries-host.mjs'), entry, directory, host],
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
      const run = {
        id,
        status: child.status,
        signal: child.signal,
        error: child.error
          ? { name: child.error.name, message: child.error.message }
          : null,
      }
      try {
        assert.equal(child.error, undefined)
        assert.equal(child.signal, null)
        assert.equal(child.status, 0)
        const complete = JSON.parse(
          readFileSync(join(directory, 'complete.json'), 'utf8'),
        )
        assert.ok(complete.ids.length > 0)
        assert.equal(new Set(complete.ids).size, complete.ids.length)
        const bytes = readFileSync(join(directory, 'records.jsonl'))
        assert.ok(bytes.length > 0)
        if (reference === undefined) reference = bytes
        assert.ok(
          bytes.equals(reference),
          `${id}: complete raw record Buffer differs`,
        )
        run.parity = true
      } catch (error) {
        assert.ok(error instanceof Error)
        run.parity = false
        run.failure = error.message
      }
      runs.push(run)
      save(`${id}.json`, run)
    }
  }
}
save('summary.json', { children: runs.length, runs })
assert.equal(runs.length, 12)
assert.ok(
  runs.every((run) => run.parity),
  'Functional/parity failures; inspect retained evidence',
)
