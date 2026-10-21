import assert from 'node:assert/strict'
import { test } from 'node:test'

import { CommandExitError, runGuarded } from './guarded-process.mjs'
import { processRuntime } from './process-inspection.mjs'

test('records a real successful command through the process surface', async () => {
  const results = []
  const result = await runGuarded(
    process.execPath,
    ['-e', 'console.log("surface-ok")'],
    {
      onResult: (value) => results.push(value),
    },
  )
  assert.equal(result.code, 0)
  assert.deepEqual(result.errors, [])
  assert.equal(results.length, 1)
})

test('distinguishes real compiler exit one from watchdog failures', async () => {
  await assert.rejects(
    runGuarded(process.execPath, ['-e', 'process.exitCode=1']),
    (error) =>
      error instanceof CommandExitError &&
      error.result.code === 1 &&
      error.result.errors.length === 0,
  )
})

test(
  'drains a real root and its observed lingering child when cancelled',
  { timeout: 30000 },
  async () => {
    const controller = new AbortController()
    let descendant
    let identity
    let output = ''
    const results = []
    const runtime = {
      ...processRuntime,
      snapshot: async () => {
        const rows = await processRuntime.snapshot()
        identity = rows.find((row) => row.pid === descendant)
        if (identity) controller.abort()
        return rows
      },
    }
    await assert.rejects(
      runGuarded(
        process.execPath,
        [
          '-e',
          `
    const {spawn}=require('node:child_process');
    const child=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});
    console.log(child.pid);
    setInterval(()=>{},1000);
  `,
        ],
        {
          runtime,
          signal: controller.signal,
          pollIntervalMs: 20,
          timeoutMs: 10000,
          onOutput: (chunk) => {
            output += chunk
            descendant = Number(output.trim())
          },
          onResult: (result) => results.push(result),
        },
      ),
      (error) =>
        !(error instanceof CommandExitError) &&
        /command cancelled/.test(error.message),
    )
    assert.ok(identity, 'descendant generation must actually be observed')
    assert.equal(results[0].stopReason, 'command cancelled')
    assert.deepEqual(results[0].errors, [])
    assert.equal(await processRuntime.inspect(descendant), undefined)
  },
)
