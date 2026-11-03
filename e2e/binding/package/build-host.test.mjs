import assert from 'node:assert/strict'
import { resolve } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'

import {
  buildHost,
  HostBuildTargetError,
  selectHostTarget,
} from './build-host.mjs'
import { targets } from './matrix.mjs'

const root = fileURLToPath(new URL('../../../', import.meta.url))

for (const row of targets) {
  test(`selects canonical host triple for ${row.suffix}`, () => {
    // Given
    let reports = 0
    const host = {
      platform: row.os,
      arch: row.cpu,
      report() {
        reports++
        return {
          header: row.libc === 'glibc' ? { glibcVersionRuntime: '2.39' } : {},
        }
      },
    }
    // When
    const selected = selectHostTarget(host)
    // Then
    assert.equal(selected, row)
    assert.equal(reports, row.os === 'linux' ? 1 : 0)
  })
}

for (const [platform, arch] of [
  ['freebsd', 'x64'],
  ['win32', 'ia32'],
  ['linux', 'riscv64'],
]) {
  test(`rejects unsupported ${platform}/${arch} before any spawn`, () => {
    // Given
    const calls = []
    const host = { platform, arch, report: () => calls.push('report') }
    const io = {
      run: () => calls.push('run'),
      now: () => calls.push('clock'),
      emit: () => calls.push('emit'),
    }
    // When / Then
    assert.throws(() => buildHost(host, io), HostBuildTargetError)
    assert.deepEqual(calls, [])
  })
}

test('preserves a Linux report failure without spawning', () => {
  // Given
  const failure = new Error('report unavailable')
  const calls = []
  const host = {
    platform: 'linux',
    arch: 'arm64',
    report() {
      throw failure
    },
  }
  const io = {
    run: () => calls.push('run'),
    now: () => 0,
    emit: () => calls.push('emit'),
  }
  // When / Then
  assert.throws(
    () => buildHost(host, io),
    (error) => error instanceof HostBuildTargetError && error.cause === failure,
  )
  assert.deepEqual(calls, [])
})

for (const report of [null, '', {}, { header: null }, { header: 'bad' }]) {
  test(`rejects indeterminate report ${JSON.stringify(report)}`, () => {
    // Given
    const host = { platform: 'linux', arch: 'x64', report: () => report }
    // When / Then
    assert.throws(() => selectHostTarget(host), HostBuildTargetError)
  })
}

test('delegates to the existing driver and emits measured completion', () => {
  // Given
  const calls = []
  const records = []
  let ticks = 0
  const host = {
    platform: 'win32',
    arch: 'arm64',
    report() {
      assert.fail('non-Linux host')
    },
  }
  const io = {
    run: (...args) => calls.push(args),
    now: () => 100 + ticks++ * 75,
    emit: (record) => records.push(record),
  }
  // When
  buildHost(host, io)
  // Then
  assert.deepEqual(calls, [
    [
      process.execPath,
      [
        resolve(root, 'bindings/devup-ui-binding/build.mjs'),
        '--target',
        'aarch64-pc-windows-msvc',
      ],
      {
        cwd: root,
        env: process.env,
        stdio: 'inherit',
      },
    ],
  ])
  assert.equal(records[0].phase, 'start')
  assert.deepEqual(records[1], {
    ...records[0],
    phase: 'finish',
    status: 'success',
    elapsedMs: 75,
  })
  assert.equal(records[1].host, 'win32/arm64')
  assert.equal(records[1].target, 'aarch64-pc-windows-msvc')
})

test('propagates driver failure and emits a measured failed finish', () => {
  // Given
  const failure = new Error('driver failed')
  const records = []
  let ticks = 0
  const host = {
    platform: 'darwin',
    arch: 'x64',
    report() {
      assert.fail('non-Linux host')
    },
  }
  const io = {
    run() {
      throw failure
    },
    now: () => ticks++ * 25,
    emit: (record) => records.push(record),
  }
  // When / Then
  assert.throws(
    () => buildHost(host, io),
    (error) => error === failure,
  )
  assert.deepEqual(records[1], {
    ...records[0],
    phase: 'finish',
    status: 'failed',
    elapsedMs: 25,
  })
})
