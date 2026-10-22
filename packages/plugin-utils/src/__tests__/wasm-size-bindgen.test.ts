import { readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { cli, validateMetadata } from '../../../../wasm-size.js'
import { sizeFixture } from './wasm-size-fixture'

const roots: string[] = []

function fixture() {
  const input = sizeFixture(roots)
  input.metadata.base.tools.wasmBindgen =
    'wasm-bindgen 0.2.128\n/base/bin/wasm-bindgen\nsha256:base'
  input.metadata.candidate.wasmBindgenVersion = '0.2.129'
  input.metadata.candidate.tools.wasmBindgen =
    'wasm-bindgen 0.2.129\n/candidate/bin/wasm-bindgen\nsha256:candidate'
  return input
}

afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

describe('per-tree bindgen ABI provenance', () => {
  it('retains both CLI versions in passing reports when locked ABIs differ', () => {
    const input = fixture()
    const manifest = join(input.root, 'metadata.json')
    const report = join(input.root, 'report.json')
    const markdown = join(input.root, 'summary.md')
    writeFileSync(manifest, JSON.stringify(input.metadata))
    const status = cli([
      '--base-pkg',
      input.base,
      '--candidate-pkg',
      input.candidate,
      '--metadata',
      manifest,
      '--report',
      report,
      '--summary',
      markdown,
    ])
    expect(status).toBe(0)
    const result: unknown = JSON.parse(readFileSync(report, 'utf8'))
    expect(result).toMatchObject({ passed: true, metadata: input.metadata })
    const summary = readFileSync(markdown, 'utf8')
    expect(summary).toContain(
      JSON.stringify(input.metadata.base.tools.wasmBindgen),
    )
    expect(summary).toContain(
      JSON.stringify(input.metadata.candidate.tools.wasmBindgen),
    )
  })

  it('accepts identical CLI records with matching equal ABIs and CRLF provenance', () => {
    const input = fixture()
    input.metadata.candidate.wasmBindgenVersion = '0.2.128'
    input.metadata.base.tools.wasmBindgen =
      'wasm-bindgen 0.2.128\r\n/shared/bin/wasm-bindgen\r\nsha256:shared'
    input.metadata.candidate.tools.wasmBindgen =
      input.metadata.base.tools.wasmBindgen
    const result = validateMetadata(input.metadata, input)
    expect(result).toEqual(input.metadata)
  })

  it('rejects different CLI records when the committed ABI is equal', () => {
    const input = fixture()
    input.metadata.candidate.wasmBindgenVersion = '0.2.128'
    input.metadata.candidate.tools.wasmBindgen =
      'wasm-bindgen 0.2.128\n/candidate/bin/wasm-bindgen\nsha256:candidate'
    expect(() => validateMetadata(input.metadata, input)).toThrow(
      'tool mismatch: wasmBindgen',
    )
  })

  it.each(['base', 'candidate'] as const)(
    'rejects a CLI version inconsistent with the %s committed lock',
    (side) => {
      const input = fixture()
      input.metadata[side].wasmBindgenVersion = '0.2.130'
      expect(() => validateMetadata(input.metadata, input)).toThrow(
        'tool mismatch',
      )
    },
  )

  it.each(['', 'latest', '0.2', '0.2.129\n'])(
    'rejects invalid declared versions when given %j',
    (version) => {
      const input = fixture()
      input.metadata.candidate.wasmBindgenVersion = version
      expect(() => validateMetadata(input.metadata, input)).toThrow()
    },
  )

  it('rejects missing lock provenance when metadata uses the old schema', () => {
    const input = fixture()
    const { commit, tree, packageDir, tools } = input.metadata.candidate
    const candidate = { commit, tree, packageDir, tools }
    const metadata = { ...input.metadata, candidate }
    expect(() => validateMetadata(metadata, input)).toThrow('keys')
  })

  it.each([
    '',
    'wasm-bindgen 0.2.128',
    'path\nwasm-bindgen 0.2.129',
    'wasm-bindgen 0.2.129 extra',
    'wasm-bindgen 0.2.1290',
  ])(
    'rejects missing or wrong actual CLI version records when given %j',
    (record) => {
      const input = fixture()
      input.metadata.candidate.tools.wasmBindgen = record
      expect(() => validateMetadata(input.metadata, input)).toThrow()
    },
  )

  it.each(['rustc', 'wasmPack', 'wasmOpt', 'bun', 'node'] as const)(
    'still rejects %s mismatches when bindgen ABIs differ',
    (tool) => {
      const input = fixture()
      input.metadata.candidate.tools[tool] = 'different binary'
      expect(() => validateMetadata(input.metadata, input)).toThrow(
        `tool mismatch: ${tool}`,
      )
    },
  )

  it.each(['rustc', 'wasmPack', 'wasmOpt', 'bun', 'node'] as const)(
    'still rejects blank %s provenance when bindgen ABIs differ',
    (tool) => {
      const input = fixture()
      input.metadata.candidate.tools[tool] = ' '
      expect(() => validateMetadata(input.metadata, input)).toThrow(
        `missing candidate tool ${tool}`,
      )
    },
  )
})
