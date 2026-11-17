import { spawn } from 'node:child_process'
import {
  appendFileSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { cases } from '../validation-cases.mjs'
import {
  countScenarioCalls,
  evaluate,
  parseDeclarations,
  parseOutputMembers,
  report,
} from './coverage.mjs'

// Runs the existing public-scenario and argument-validation oracles against
// the built WASM package, next to the native addon they must match, then
// reports which exports and argument positions they reached.
const here = fileURLToPath(new URL('.', import.meta.url))
const root = resolve(here, '../../..')
const [
  wasm = join(root, 'bindings/devup-ui-wasm/pkg/index.js'),
  native = join(root, 'bindings/devup-ui-binding/pkg/index.cjs'),
] = process.argv.slice(2)
const evidence = mkdtempSync(join(tmpdir(), 'devup-wasm-abi-'))

function run(script, directory) {
  const args = [
    join(here, '..', script),
    wasm,
    native,
    join(evidence, directory),
  ]
  return new Promise((done) => {
    const child = spawn(process.execPath, args, {
      env: { ...process.env, NODE_OPTIONS: '', BUN_OPTIONS: '' },
      stdio: ['ignore', 'inherit', 'inherit'],
    })
    child.on('error', (error) => done({ script, error: error.message }))
    child.on('exit', (status, signal) => done({ script, status, signal }))
  })
}

// identity.mjs: the public corpus on both engines, three times each, compared
// byte for byte. validation.mjs: every invalid argument shape on both engines
// rejected with the same TypeError and no state change.
const runs = await Promise.all([
  run('identity.mjs', 'identity'),
  run('validation.mjs', 'validation'),
])
const failed = runs.filter((result) => result.status !== 0)

const records = readFileSync(
  join(evidence, 'identity/wasm-1/records.jsonl'),
  'utf8',
)
  .trimEnd()
  .split('\n')
  .map((line) => JSON.parse(line))
const dts = readFileSync(join(dirname(wasm), 'index.d.ts'), 'utf8')
const result = evaluate({
  declarations: parseDeclarations(dts),
  members: parseOutputMembers(dts),
  calls: countScenarioCalls(records),
  cases,
})
const markdown = report(result)
console.info(markdown)
writeFileSync(
  join(evidence, 'abi-coverage.json'),
  `${JSON.stringify({ runs, ...result }, null, 2)}\n`,
)
if (process.env.GITHUB_STEP_SUMMARY)
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, markdown)
console.info(`Evidence: ${evidence}`)
for (const { script } of failed) console.error(`${script} failed`)
if (failed.length || !result.ok) process.exitCode = 1
