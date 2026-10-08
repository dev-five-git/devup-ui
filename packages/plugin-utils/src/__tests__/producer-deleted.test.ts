import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { expect, it, spyOn } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { object, text } from '../../../../test-harness/producer-data'
import { decodeMap } from '../../../../test-harness/producer-map'
import {
  reconstructNative,
  validateNative,
} from '../../../../test-harness/producer-native'
import { captureProcess } from '../../../../test-harness/producer-process'
import { ProducerProtocol } from '../../../../test-harness/producer-protocol'
import { parseTestResult, runTestGroups } from '../../../../test-harness/run'

function deletedFixture(extension: 'cjs' | 'mjs', partial = false) {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-deleted-'))
  const modules = mkdtempSync(join(tmpdir(), 'devup-deleted-modules-'))
  const config = join(root, 'bunfig.toml')
  const sources = {
    cjs: {
      'plugin.cjs':
        'exports.value = function value(input) { return input + 1 }\n',
      'loader.cjs':
        "const plugin = require('./plugin.cjs'); exports.load = function load(input) { return plugin.value(input) }\n",
      'caller.cjs':
        "const loader = require('./loader.cjs'); exports.modifyConfig = function modifyConfig(input) { return loader.load(input) }\n",
    },
    mjs: {
      'plugin.mjs': 'export function value(input) { return input + 1 }\n',
      'loader.mjs':
        "import {value} from './plugin.mjs'; export function load(input) { return value(input) }\n",
      'caller.mjs':
        "import {load} from './loader.mjs'; export function modifyConfig(input) { return load(input) }\n",
    },
  } as const
  const inputs = Object.entries(sources[extension]).map(([name, source]) => ({
    path: join(modules, name),
    source:
      source +
      (partial && name.startsWith('plugin')
        ? 'export function unused() { return 99 }\n'
        : ''),
  }))
  writeFileSync(
    config,
    '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\ncoverageSkipTestFiles=true\n',
  )
  const caller = JSON.stringify(join(modules, `caller.${extension}`))
  const load = {
    cjs: `createRequire(import.meta.url)(${caller})`,
    mjs: `await import(${caller}+'?first')`,
  }
  const reload = {
    cjs: '',
    mjs: `const last=await import(${caller}+'?last');expect(last.modifyConfig(41)).toBe(42);`,
  }
  writeFileSync(
    join(root, 'source.test.ts'),
    `import {it,expect} from 'bun:test';import {mkdirSync,writeFileSync,rmSync} from 'node:fs';import {createRequire} from 'node:module';it('write load delete',async()=>{mkdirSync(${JSON.stringify(modules)},{recursive:true});for(const input of ${JSON.stringify(inputs)})writeFileSync(input.path,input.source);try{const caller=${load[extension]};expect(caller.modifyConfig(41)).toBe(42);${reload[extension]}}finally{rmSync(${JSON.stringify(modules)},{recursive:true});Bun.gc(true)}})`,
  )
  return { root, modules, config, inputs }
}

it.each(['cjs', 'mjs'] as const)(
  'binds actual %s caller loader and plugin evidence when teardown deletes their files before final capture',
  async (extension) => {
    // Given: the child writes and loads real modules, then deletes them and runs GC.
    const fixture = deletedFixture(extension)
    try {
      // When: the real producer captures its final public/native lifetime.
      const result = await captureProcess({
        root: fixture.root,
        config: fixture.config,
        coverage: join(fixture.root, 'coverage'),
        group: 'isolated',
        files: ['source.test.ts'],
        executable: process.execPath,
      })
      const bound = VerifiedSourceCoverage.bind(result, fixture.root)
      // Then: deleted inputs retain actual sourceID, emitted code/map and native rows.
      expect(result.value.status).toBe(0)
      const witnesses = fixture.inputs.map((input) => {
        const captures = result.value.captures.filter(
          (item) => item.url === input.path,
        )
        const capture = captures.at(-1)
        const source = parseLcov(result.value.lcov).find(
          (item) => resolve(fixture.root, item.file) === input.path,
        )
        if (!capture || !source)
          throw new CoverageError('missing deleted native witness')
        expect(decodeMap(capture.code, capture.sourceMapURL).input).toBe(
          input.source,
        )
        expect(capture.blocks.length).toBeGreaterThan(0)
        expect(
          captures.every(
            (item) =>
              item.code === capture.code &&
              item.sourceMapURL === capture.sourceMapURL,
          ),
        ).toBe(true)
        expect(() =>
          validateNative(reconstructNative(capture, source), source),
        ).not.toThrow()
        return {
          capture,
          source: {
            file: source.file,
            functions: [source.foundFunctions, source.hitFunctions],
            lines: [...source.lines],
          },
        }
      })
      expect(() =>
        auditCoverage(
          bound,
          witnesses.map((item) => item.source.file),
        ),
      ).not.toThrow()
      console.info(
        JSON.stringify({
          case: 'actual-deleted-producer',
          extension,
          pid: result.value.pid,
          bun: result.value.bun,
          revision: result.value.revision,
          witnesses,
        }),
      )
    } finally {
      rmSync(fixture.root, { recursive: true, force: true })
      rmSync(fixture.modules, { recursive: true, force: true })
    }
  },
  120000,
)

it('publishes both real groups when each recreates loads and deletes the same ESM chain', async () => {
  // Given: each real group exercises identical effective modules before teardown.
  const fixture = deletedFixture('mjs')
  const output = spyOn(console, 'info').mockImplementation(() => undefined)
  try {
    const required = fixture.inputs.map((input) =>
      relative(fixture.root, input.path).replaceAll('\\', '/'),
    )
    // When: the unchanged root merge and strict per-source auditor publish them.
    const summary = await runTestGroups({
      root: fixture.root,
      config: fixture.config,
      groups: { isolated: ['source.test.ts'], runtime: ['source.test.ts'] },
      required,
    })
    // Then: full-universe output includes every deleted module at strict100.
    expect(parseTestResult(summary)).toEqual({ pass: 2, fail: 0 })
    const sources = parseLcov(
      readFileSync(join(fixture.root, 'coverage/lcov.info'), 'utf8'),
    )
    expect(
      sources.filter((source) => required.includes(source.file)).length,
    ).toBe(3)
  } finally {
    output.mockRestore()
    rmSync(fixture.root, { recursive: true, force: true })
    rmSync(fixture.modules, { recursive: true, force: true })
  }
}, 120000)

it('rejects incomplete native coverage when an uncalled function belongs to a deleted module', async () => {
  const fixture = deletedFixture('mjs', true)
  try {
    // Given / When: actual loaded deleted code contains an unexecuted function.
    const result = await captureProcess({
      root: fixture.root,
      config: fixture.config,
      coverage: join(fixture.root, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    const bound = VerifiedSourceCoverage.bind(result, fixture.root)
    // Then: teardown cannot waive any native function or zero-hit row.
    expect(() => auditCoverage(bound, [])).toThrow(
      'below 100% lines/functions:',
    )
  } finally {
    rmSync(fixture.root, { recursive: true, force: true })
    rmSync(fixture.modules, { recursive: true, force: true })
  }
}, 120000)

it('rejects missing repository capture when the actual inspector transport loses its parsed URL', async () => {
  // Given: a genuine repository module executes; transport corruption omits only its URL.
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-repo-missing-'))
  const source = fileURLToPath(
    new URL('../../../../test-harness/groups.ts', import.meta.url),
  )
  const original = ProducerProtocol.prototype.request
  let suppressed = 0
  const transport = spyOn(
    ProducerProtocol.prototype,
    'request',
  ).mockImplementation(async function (
    this: ProducerProtocol,
    method,
    params = {},
  ) {
    const response = await original.call(this, method, params)
    if (method === 'Inspector.initialized') {
      this.onEvent = (event) => {
        if (event['method'] !== 'Debugger.scriptParsed') return
        const parsed = object(event['params'])
        if (
          text(parsed['url']).replaceAll('\\', '/') ===
          source.replaceAll('\\', '/')
        ) {
          parsed['url'] = ''
          suppressed++
        }
      }
    }
    return response
  })
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      `import {it,expect} from 'bun:test';import {parseTestGroup} from ${JSON.stringify(source)};it('repository',()=>expect(parseTestGroup('isolated')).toBe('isolated'))`,
    )
    // When: the capturer issues only actual observed evidence, never a fabricated receipt.
    const result = await captureProcess({
      root,
      config,
      coverage: join(root, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    // Then: LCOV still includes the repository source and bind refuses specifically its gap.
    expect(result.value.status).toBe(0)
    expect(suppressed).toBe(1)
    expect(
      parseLcov(result.value.lcov).some(
        (item) => resolve(root, item.file) === source,
      ),
    ).toBe(true)
    const bind = () => VerifiedSourceCoverage.bind(result, root)
    expect(bind).toThrow(CoverageError)
    expect(bind).toThrow('missing producer capture')
  } finally {
    transport.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
