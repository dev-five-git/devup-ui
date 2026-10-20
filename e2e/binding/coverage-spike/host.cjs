'use strict'
const assert = require('node:assert/strict')
const { realpathSync } = require('node:fs')
const { extname, isAbsolute } = require('node:path')

// Given: exactly one explicit scenario selector and an absolute native image.
const control = process.env.W74A_CONTROL
assert.ok(
  control === 'negative' || control === 'positive',
  'W74A_CONTROL required',
)
const requested = process.env.W74A_ADDON
assert.equal(typeof requested, 'string', 'W74A_ADDON required')
assert.ok(isAbsolute(requested), 'W74A_ADDON must be absolute')
assert.equal(extname(requested), '.node')
const loaded = realpathSync(requested)
console.info(
  JSON.stringify({
    event: 'W74a.host',
    control,
    LLVM_PROFILE_FILE: process.env.LLVM_PROFILE_FILE ?? null,
    requestedAddon: requested,
    loadedAddon: loaded,
  }),
)
const addon = require(loaded)

// When/Then: exercise success/error and a real Env conversion through JS only.
assert.equal(addon.converted(42), 42)
assert.throws(() => addon.converted(-1), { code: 'InvalidArg' })
assert.equal(
  addon.callbackProbe(() => 73),
  73,
)
assert.equal(
  addon.callbackProbe(() => {
    throw new Error('W74a callback miss')
  }),
  null,
)
assert.equal(
  addon.callbackProbe(() => 'invalid return'),
  null,
)
assert.equal(addon.converted(0), 0) // A throw must not leave an exception pending.
const probe = new addon.Probe('W74a native allocation')
assert.equal(probe.value, 'W74a native allocation')
assert.equal(probe.free(), true)
assert.throws(() => probe.value, { code: 'InvalidArg' })
assert.equal(probe.free(), false)

// Same native object/source in both controls; only this invocation differs.
switch (control) {
  case 'negative':
    break
  case 'positive':
    assert.equal(addon.controlOnly(7), 0x772)
    break
  default:
    assert.fail('unreachable W74A_CONTROL')
}
// No process.exit(), GC polling, timers, detached work or profile mutation.
