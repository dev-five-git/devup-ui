'use strict'
const assert = require('node:assert/strict')
const { readFileSync } = require('node:fs')
const { join } = require('node:path')
const { test } = require('node:test')
const { runInNewContext } = require('node:vm')
const source = readFileSync(join(__dirname, 'loader.cjs'), 'utf8')

// Foreign host simulation proves dispatch only, never native execution.
function fixture(platform, arch, behavior = {}) {
  const calls = []
  const raw = {}
  const missing = Object.assign(new Error('fixture resolution failure'), {
    code: 'MODULE_NOT_FOUND',
  })
  const require = (name) => {
    calls.push(['load', name])
    if (behavior.loadError) throw behavior.loadError
    return raw
  }
  require.resolve = (name) => {
    calls.push(['resolve', name])
    if (behavior.resolveError) throw behavior.resolveError
    if (behavior.missing) throw missing
    return name
  }
  const module = { exports: {} }
  const process = {
    platform,
    arch,
    report: {
      getReport() {
        calls.push(['report'])
        if (behavior.reportError) throw behavior.reportError
        return { header: behavior.musl ? {} : { glibcVersionRuntime: '2.39' } }
      },
    },
  }
  return {
    calls,
    raw,
    missing,
    run() {
      runInNewContext(
        source,
        { module, require, process },
        { filename: join(__dirname, 'loader.cjs') },
      )
      return module.exports
    },
  }
}

for (const platform of ['win32', 'darwin', 'linux']) {
  for (const arch of ['x64', 'arm64']) {
    for (const musl of platform === 'linux' ? [false, true] : [false]) {
      test(`selects ${platform}/${arch}/${musl ? 'musl' : 'gnu'} once`, () => {
        // Given
        const host = fixture(platform, arch, { musl })
        const suffix =
          platform === 'linux'
            ? `-${musl ? 'musl' : 'gnu'}`
            : platform === 'win32'
              ? '-msvc'
              : ''
        const name = `@devup-ui/binding-${platform}-${arch}${suffix}`
        // When
        const result = host.run()
        // Then: only the selected package is requested, with Linux-only libc inspection.
        assert.equal(result, host.raw)
        assert.deepEqual(host.calls, [
          ...(platform === 'linux' ? [['report']] : []),
          ['resolve', `${name}/package.json`],
          ['load', name],
        ])
      })
    }
  }
}
for (const [platform, arch] of [
  ['freebsd', 'x64'],
  ['win32', 'ia32'],
  ['linux', 'riscv64'],
]) {
  test(`rejects unsupported ${platform}/${arch} without loading`, () => {
    const host = fixture(platform, arch)
    assert.throws(
      () => host.run(),
      (error) => error.message.includes(`${platform}/${arch}/`),
    )
    assert.deepEqual(host.calls, platform === 'linux' ? [['report']] : [])
  })
}
test('names omitted optional package and preserves resolution cause', () => {
  const host = fixture('win32', 'x64', { missing: true })
  assert.throws(
    () => host.run(),
    (error) => {
      assert.equal(error.cause, host.missing)
      assert.match(error.message, /@devup-ui\/binding-win32-x64-msvc/)
      assert.match(error.message, /optional dependencies enabled/)
      return true
    },
  )
  assert.equal(host.calls.length, 1)
})
test('preserves genuine manifest resolution failure', () => {
  const failure = Object.assign(new Error('access denied'), { code: 'EACCES' })
  const host = fixture('darwin', 'arm64', { resolveError: failure })
  assert.throws(
    () => host.run(),
    (error) => error === failure,
  )
  assert.equal(host.calls.length, 1)
})
for (const code of ['ERR_DLOPEN_FAILED', 'MODULE_NOT_FOUND']) {
  test(`preserves found package load failure ${code} without fallback`, () => {
    const failure = Object.assign(
      new Error('broken image or transitive module'),
      { code },
    )
    const host = fixture('linux', 'x64', { loadError: failure })
    assert.throws(
      () => host.run(),
      (error) => error === failure,
    )
    assert.equal(host.calls.length, 3)
  })
}
test('preserves libc inspection failure without guessing a target', () => {
  const failure = new Error('report unavailable')
  const host = fixture('linux', 'arm64', { reportError: failure })
  assert.throws(
    () => host.run(),
    (error) => error === failure,
  )
  assert.deepEqual(host.calls, [['report']])
})
