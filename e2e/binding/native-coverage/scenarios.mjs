import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  closeSync,
  mkdirSync,
  openSync,
  readFileSync,
  writeFileSync,
} from 'node:fs'
import { dirname, isAbsolute, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { cases } from '../validation-cases.mjs'

const [entry, evidence] = process.argv.slice(2)
assert.equal(process.argv.length, 4)
assert.ok(isAbsolute(entry) && isAbsolute(evidence))
assert.equal(process.versions.bun, undefined, 'Rust coverage uses real Node')
const here = dirname(fileURLToPath(import.meta.url))
const runs = []
const run = (name, script, args) => {
  const directory = join(evidence, name)
  mkdirSync(directory)
  const stdout = openSync(join(directory, 'stdout.log'), 'wx')
  const stderr = openSync(join(directory, 'stderr.log'), 'wx')
  let child
  try {
    child = spawnSync(
      process.execPath,
      [join(here, script), entry, directory, ...args],
      {
        env: { ...process.env, NODE_OPTIONS: '', BUN_OPTIONS: '' },
        stdio: ['ignore', stdout, stderr],
        timeout: 120000,
      },
    )
  } finally {
    closeSync(stdout)
    closeSync(stderr)
  }
  const result = {
    name,
    pid: child.pid,
    status: child.status,
    signal: child.signal,
    error: child.error?.message,
  }
  runs.push(result)
  writeFileSync(
    join(directory, 'process.json'),
    JSON.stringify(result, null, 2) + '\n',
    { flag: 'wx' },
  )
  assert.equal(child.error, undefined)
  assert.equal(child.signal, null)
  assert.equal(child.status, 0, `${name}: see ${directory}`)
  return directory
}

const entries = run('entries', '../entries-host.mjs', ['node'])
const complete = JSON.parse(
  readFileSync(join(entries, 'complete.json'), 'utf8'),
)
assert.ok(complete.ids.length >= 171)
assert.equal(new Set(complete.ids).size, complete.ids.length)
run('public', '../host.mjs', [])
for (const test of cases)
  run(`validation-${test.id}`, '../validation-host.mjs', [test.id])
for (const group of ['protocol', 'raw-resolver-failure'])
  run(`private-${group}`, 'private-host.mjs', [group])
writeFileSync(
  join(evidence, 'complete.json'),
  JSON.stringify({ entry, node: process.execPath, runs }, null, 2) + '\n',
  { flag: 'wx' },
)
