import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

export function sizeFixture(roots: string[]) {
  const root = mkdtempSync(join(tmpdir(), 'wasm-size-'))
  roots.push(root)
  const base = join(root, 'base')
  const candidate = join(root, 'candidate')
  for (const directory of [base, candidate]) {
    mkdirSync(directory)
    writeFileSync(join(directory, 'index_bg.wasm'), 'abc')
    writeFileSync(join(directory, 'index.js'), 'export const answer = 42\n')
  }
  const tools = {
    rustc: 'rustc fixture',
    wasmPack: 'wasm-pack fixture',
    wasmBindgen: 'wasm-bindgen 0.2.128',
    wasmOpt: 'wasm-opt fixture',
    bun: 'bun fixture',
    node: 'node fixture',
  }
  const metadata = {
    jobId: 'fixture-job',
    buildCommand: 'fixture release build and postprocess',
    base: {
      commit: 'a'.repeat(40),
      tree: 'b'.repeat(40),
      packageDir: base,
      wasmBindgenVersion: '0.2.128',
      tools: { ...tools },
    },
    candidate: {
      commit: 'c'.repeat(40),
      tree: 'd'.repeat(40),
      packageDir: candidate,
      wasmBindgenVersion: '0.2.128',
      tools: { ...tools },
    },
  }
  const manifest = join(root, 'metadata.json')
  writeFileSync(manifest, JSON.stringify(metadata))
  const report = join(root, 'report.json')
  const markdown = join(root, 'summary.md')
  const args = [
    '--base-pkg',
    base,
    '--candidate-pkg',
    candidate,
    '--metadata',
    manifest,
    '--report',
    report,
  ]
  return { root, base, candidate, metadata, manifest, report, markdown, args }
}
