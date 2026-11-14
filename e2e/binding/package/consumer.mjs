import assert from 'node:assert/strict'
import {
  existsSync,
  lstatSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { targets } from './matrix.mjs'
import { packageDirectory, packageName, verifyTarball } from './pack.mjs'
import {
  command,
  direct,
  hashFile,
  readJson,
  root,
  tool,
  writeJson,
} from './provenance.mjs'
import { child, installConsumer, registry } from './registry.mjs'

// This source runs from a clean install, never from repository resolution paths.
const probe = `
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync, realpathSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { rawDiagnostics, runDiagnostics } from './diagnostics.mjs'
import * as esm from '@devup-ui/binding'
const require = createRequire(import.meta.url)
const cjs = require('@devup-ui/binding')
const expected = JSON.parse(readFileSync(new URL('./expected.json', import.meta.url)))
assert.deepEqual(Object.keys(cjs).sort(), expected.names)
assert.deepEqual(Object.keys(esm).sort(), expected.names)
for (const name of expected.names) assert.equal(esm[name], cjs[name], name)
for (const name of ['loader.cjs', 'adapter.cjs', 'pkg/index.cjs', ...(process.versions.bun ? [] : ['package.json'])]) {
  assert.throws(() => require('@devup-ui/binding/' + name), (error) =>
    error.code === (process.versions.bun ? 'MODULE_NOT_FOUND' : 'ERR_PACKAGE_PATH_NOT_EXPORTED'))
}
cjs.resetStateForTesting()
assert.throws(() => cjs.setPrefix(false), TypeError)
cjs.setModuleResolver((specifier) => specifier === './color' ? { path: '/color.ts', code: 'export const color = "red"' } : undefined)
const output = esm.codeExtract('/fixture.tsx', 'import { Box } from "@devup-ui/react"; import { color } from "./color"; export const App = () => <Box bg={color} />', '@devup-ui/react', 'df', true, false, false, {})
assert.ok(output instanceof cjs.Output)
assert.match(output.code, /className/)
assert.match(cjs.getCss(null, false), /red/)
assert.deepEqual(output.dependencies, ['/color.ts'])
output.dependencies.push('not-native')
assert.deepEqual(output.dependencies, ['/color.ts'])
assert.equal(typeof output.map, 'string')
output[Symbol.dispose]()
const noMap = cjs.codeExtractWithoutSourceMap('/plain.tsx', 'export const x = 1', '@devup-ui/react', 'df', true, false, false, {})
assert.equal(noMap.map, undefined)
noMap.free()
cjs.resetBuildState()
assert.doesNotMatch(cjs.getCss(null, false), /background:red/)
cjs.resetStateForTesting()
const image = require.resolve(expected.sibling)
const entry = require.resolve('@devup-ui/binding')
const sha256 = createHash('sha256').update(readFileSync(image)).digest('hex')
assert.equal(sha256, expected.imageHash)
for (const path of [entry, image]) assert.ok(realpathSync(path).startsWith(realpathSync(new URL('.', import.meta.url))))
const diagnostics = runDiagnostics(cjs)
console.log(JSON.stringify({ runtime: process.versions, executable: process.execPath, entry, image, sha256, names: expected.names.length, diagnostics, rawDiagnostics, result: 'PASS' }))
`
const declarations = `
binding.setModuleResolver((specifier: string, importer: string) => ({ path: importer, code: specifier }))
binding.setModuleResolver(null)
binding.setPrefix(undefined)
binding.setAtomHoist(null)
const output: binding.Output = binding.codeExtract('x.tsx', '', '@devup-ui/react', 'df', true, false, false, {})
const code: string = output.code
const css: string | undefined = output.css
const dependencies: string[] = output.dependencies
output.free()
output[Symbol.dispose]()
void [code, css, dependencies]
`
export async function consume(options) {
  const bundle = readJson(join(options.bundle, 'bundle.json'))
  assert.equal(bundle.status, 'verified')
  assert.ok(
    options.localHostOnly || bundle.mode === 'FULL nine-pack',
    'partial bundle forbidden by default',
  )
  assert.equal(bundle.packs.length, options.localHostOnly ? 2 : 9)
  const row = targets.find((item) => item.suffix === options.suffix)
  assert.ok(
    row && row.os === process.platform && row.cpu === process.arch,
    'consumer must execute on matching host',
  )
  if (row.os === 'linux')
    assert.equal(
      row.libc,
      process.report.getReport().header.glibcVersionRuntime ? 'glibc' : 'musl',
    )
  const host = bundle.packs.find((pack) => pack.suffix === row.suffix)
  assert.ok(host, 'missing matching target pack')
  for (const pack of bundle.packs) {
    const actual = verifyTarball(join(options.bundle, pack.file), {
      version: pack.version,
      row: targets.find((item) => item.suffix === pack.suffix),
      image: pack.image,
    })
    assert.deepEqual(actual, pack, 'bundle tarball/metadata drift')
  }
  const directory = mkdtempSync(join(tmpdir(), 'native-clean-consumer-'))
  const report = {
    status: 'running',
    directory,
    mode: bundle.mode,
    runtimeHost: { platform: process.platform, arch: process.arch },
    tools: {
      node: tool('node'),
      bun: tool('bun'),
      npm: {
        executable: options.npmCli,
        sha256: hashFile(options.npmCli),
        version: command(process.execPath, [options.npmCli, '--version']),
      },
      typescript: tool(options.tsc),
    },
    installs: [],
  }
  const server = await registry(bundle, options.bundle)
  try {
    const names = [
      ...readFileSync(
        join(root, packageDirectory(), 'index.d.cts'),
        'utf8',
      ).matchAll(/export (?:class|function) (\w+)/g),
    ]
      .map((match) => match[1])
      .sort()
    assert.equal(names.length, 29)
    for (const manager of ['npm', 'bun']) {
      for (const omit of [false, true]) {
        const startRequest = server.requests.length
        const { fixture, env } = await installConsumer({
          directory,
          manager,
          omit,
          version: host.version,
          origin: server.origin,
          npmCli: options.npmCli,
        })
        const selected = targets.filter((item) =>
          existsSync(join(fixture, 'node_modules', packageName(item))),
        )
        assert.deepEqual(
          selected,
          omit ? [] : [row],
          'actual optional package selection',
        )
        const result = {
          manager,
          omit,
          fixture,
          selected: selected.map(packageName),
          requests: server.requests.slice(startRequest),
          probes: [],
        }
        report.installs.push(result)
        if (omit) {
          const check = `const assert = require('node:assert/strict'); assert.throws(() => require('@devup-ui/binding'), (e) => e.message.includes(${JSON.stringify(packageName(row))}) && e.message.includes('optional dependencies enabled'))`
          for (const runtime of ['node', 'bun'])
            result.probes.push(
              await child(runtime, ['-e', check], { cwd: fixture, env }),
            )
          continue
        }
        for (const name of ['@devup-ui/binding', packageName(row)]) {
          const path = join(fixture, 'node_modules', name)
          assert.ok(!lstatSync(path).isSymbolicLink(), 'no workspace symlinks')
          assert.ok(realpathSync(path).startsWith(realpathSync(fixture)))
        }
        writeJson(join(fixture, 'expected.json'), {
          names,
          sibling: packageName(row),
          imageHash: host.image.sha256,
          directory: realpathSync(fixture),
        })
        writeFileSync(join(fixture, 'probe.mjs'), probe)
        writeFileSync(
          join(fixture, 'diagnostics.mjs'),
          readFileSync(join(root, 'e2e/binding/diagnostics.mjs')),
        )
        for (const runtime of ['node', 'bun']) {
          const output = await child(runtime, ['probe.mjs'], {
            cwd: fixture,
            env,
          })
          result.probes.push(JSON.parse(output.stdout))
        }
        writeFileSync(
          join(fixture, 'consumer.cts'),
          `import binding = require('@devup-ui/binding')\n${declarations}`,
        )
        writeFileSync(
          join(fixture, 'consumer.mts'),
          `import * as binding from '@devup-ui/binding'\n${declarations}`,
        )
        result.types = await child(
          options.tsc,
          [
            '--noEmit',
            '--strict',
            '--noUncheckedIndexedAccess',
            '--exactOptionalPropertyTypes',
            '--module',
            'nodenext',
            '--target',
            'esnext',
            '--lib',
            'esnext',
            'consumer.cts',
            'consumer.mts',
          ],
          { cwd: fixture, env },
        )
      }
    }
    report.status = 'PASS'
    return report
  } catch (error) {
    report.status = 'failed'
    report.error = {
      message: error.message,
      stack: error.stack,
      stdout: error.stdout,
      stderr: error.stderr,
    }
    throw error
  } finally {
    report.requests = server.requests
    await server.close()
    writeJson(options.report, report)
  }
}
if (direct(import.meta.url)) {
  const [bundle, suffix, report, tsc, npmCli, ...flags] = process.argv.slice(2)
  assert.ok(flags.every((flag) => flag === '--local-host-only'))
  await consume({
    bundle: resolve(bundle),
    suffix,
    report: resolve(report),
    tsc: resolve(tsc),
    npmCli: resolve(npmCli),
    localHostOnly: flags.includes('--local-host-only'),
  })
}
