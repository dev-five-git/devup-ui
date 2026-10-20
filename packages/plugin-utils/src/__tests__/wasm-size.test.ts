import { spawnSync } from 'node:child_process'
import { readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { gzipSync } from 'node:zlib'

import { afterEach, describe, expect, it } from 'bun:test'

import config from '../../../../bindings/devup-ui-wasm/size-budget.json'
import {
  cli,
  compare,
  measure,
  summary,
  validateMetadata,
} from '../../../../wasm-size.js'
import { sizeFixture } from './wasm-size-fixture'

const roots: string[] = []
const checker = fileURLToPath(
  new URL('../../../../verify-wasm-size.js', import.meta.url),
)

const fixture = () => sizeFixture(roots)

afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

describe('WASM size measurement and provenance', () => {
  it('reads real files with gzip9 sizes and SHA256 identities', () => {
    const input = fixture()
    const result = measure(input.base)
    expect(result.wasm).toEqual({
      file: join(input.base, 'index_bg.wasm'),
      raw: 3,
      gzip: gzipSync(Buffer.from('abc'), { level: 9 }).length,
      sha256:
        'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
    })
    expect(result.glue.raw).toBe(
      Buffer.byteLength('export const answer = 42\n'),
    )
  })

  it.each(['missing', 'empty'])('fails when an artifact is %s', (state) => {
    const input = fixture()
    const path = join(input.base, 'index.js')
    if (state === 'missing') rmSync(path)
    else writeFileSync(path, '')
    expect(() => measure(input.base)).toThrow()
  })

  it.each(['commit', 'tree'] as const)('rejects missing %s identity', (id) => {
    const input = fixture()
    input.metadata.candidate[id] = ''
    expect(() => validateMetadata(input.metadata, input)).toThrow()
  })

  it.each(['jobId', 'buildCommand'] as const)(
    'rejects blank %s provenance',
    (field) => {
      const input = fixture()
      input.metadata[field] = ' '
      expect(() => validateMetadata(input.metadata, input)).toThrow()
    },
  )

  it.each(['', 'different binary'])(
    'rejects invalid tool provenance %j',
    (version) => {
      const input = fixture()
      input.metadata.candidate.tools.wasmOpt = version
      expect(() => validateMetadata(input.metadata, input)).toThrow()
    },
  )

  it.each(['', 'base'])(
    'rejects invalid package directory provenance %j',
    (directory) => {
      const input = fixture()
      input.metadata.candidate.packageDir = directory ? input.base : ''
      expect(() => validateMetadata(input.metadata, input)).toThrow()
    },
  )

  it('rejects shared package directories when both metadata paths match', () => {
    const input = fixture()
    input.metadata.candidate.packageDir = input.base
    const directories = { base: input.base, candidate: input.base }
    expect(() => validateMetadata(input.metadata, directories)).toThrow()
  })

  it('records both source identities and hashes in the summary', () => {
    const input = fixture()
    const base = measure(input.base)
    const candidate = measure(input.candidate)
    const report = {
      ...compare(base, candidate, config),
      metadata: input.metadata,
      measurements: { base, candidate },
      config,
      measurement: { level: 9 },
    }
    const markdown = summary(report)
    for (const source of [input.metadata.base, input.metadata.candidate]) {
      expect(markdown).toContain(source.commit)
      expect(markdown).toContain(source.tree)
    }
    expect(markdown).toContain(base.wasm.sha256)
    expect(markdown).toContain('| wasm/raw | 3 | 3 | 1024 | 0 | 0.000 | PASS |')
  })
})

describe('WASM size CLI', () => {
  it.each([false, true])(
    'preserves reports when comparison failure is %s',
    (failure) => {
      const input = fixture()
      if (failure)
        writeFileSync(
          join(input.candidate, 'index_bg.wasm'),
          Buffer.alloc(2048),
        )
      const args = [checker, ...input.args, '--summary', input.markdown]
      const result = spawnSync('node', args, { encoding: 'utf8' })
      expect(result.status).toBe(failure ? 1 : 0)
      const report: unknown = JSON.parse(readFileSync(input.report, 'utf8'))
      expect(report).toMatchObject({
        passed: !failure,
        metadata: input.metadata,
        measurement: { level: 9, implementation: 'node:zlib.gzipSync' },
      })
      expect(readFileSync(input.markdown, 'utf8')).toContain(
        failure ? '## WASM size: FAIL' : '## WASM size: PASS',
      )
    },
  )

  it.each([false, true])(
    'writes reports in-process with failure %s',
    (failure) => {
      const input = fixture()
      if (failure)
        writeFileSync(
          join(input.candidate, 'index_bg.wasm'),
          Buffer.alloc(2048),
        )
      const policy = join(input.root, 'policy.json')
      writeFileSync(policy, JSON.stringify(config))
      const args = [
        ...input.args,
        '--config',
        policy,
        '--summary',
        input.markdown,
      ]
      const status = cli(args)
      expect(status).toBe(failure ? 1 : 0)
      const report: unknown = JSON.parse(readFileSync(input.report, 'utf8'))
      expect(report).toMatchObject({ passed: !failure, config })
    },
  )

  it('appends when GITHUB_STEP_SUMMARY supplies the destination', () => {
    const input = fixture()
    writeFileSync(input.markdown, 'previous\n')
    const status = cli(input.args, { GITHUB_STEP_SUMMARY: input.markdown })
    expect(status).toBe(0)
    expect(readFileSync(input.markdown, 'utf8')).toStartWith(
      'previous\n## WASM size: PASS',
    )
  })

  it.each([
    { args: [] },
    { args: ['--write'] },
    { args: ['--from', 'old.json'] },
  ])('rejects absent or obsolete arguments %j', ({ args }) => {
    expect(() => cli(args, {})).toThrow()
  })

  it.each([
    'metadata',
    'config',
    'summary',
    'artifact',
    'collision',
    'overwrite',
  ] as const)('rejects invalid %s input without skipping', (invalid) => {
    const input = fixture()
    const args = [...input.args, '--summary', input.markdown]
    switch (invalid) {
      case 'metadata':
        writeFileSync(input.manifest, '{')
        break
      case 'config':
        args.push('--config', join(input.root, 'absent.json'))
        break
      case 'summary':
        args.splice(-2)
        break
      case 'artifact':
        rmSync(join(input.base, 'index_bg.wasm'))
        break
      case 'collision':
        args.splice(-1, 1, input.report)
        break
      case 'overwrite':
        args.splice(-1, 1, input.manifest)
        break
      default: {
        const exhaustive: never = invalid
        throw new Error(String(exhaustive))
      }
    }
    expect(() => cli(args, {})).toThrow()
  })
})
