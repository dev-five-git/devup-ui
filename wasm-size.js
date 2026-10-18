import { createHash } from 'node:crypto'
import {
  appendFileSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from 'node:fs'
import { join, resolve } from 'node:path'
import { parseArgs } from 'node:util'
import { gzipSync } from 'node:zlib'

import { wasmBindgenVersionPattern } from './wasm-bindgen-version.js'

const artifacts = { wasm: 'index_bg.wasm', glue: 'index.js' }
const tools = ['rustc', 'wasmPack', 'wasmBindgen', 'wasmOpt', 'bun', 'node']
const readJson = (file) => JSON.parse(readFileSync(file, 'utf8'))
const object = (value) =>
  value !== null && typeof value === 'object' && !Array.isArray(value)
const text = (value) => typeof value === 'string' && value.trim().length > 0
const bytes = (value) => Number.isSafeInteger(value) && value > 0

function requireInput(condition, message) {
  if (!condition) throw new Error(`verify-wasm-size: ${message}`)
}

function keys(value, expected, label) {
  const matches = object(value) && Object.keys(value).sort().join()
  requireInput(matches === [...expected].sort().join(), `invalid ${label} keys`)
}

export function validateConfig(config) {
  keys(config, ['wasm', 'glue'], 'config')
  for (const name of Object.keys(artifacts)) {
    keys(config[name], ['raw', 'gzip'], `config.${name}`)
    for (const kind of ['raw', 'gzip']) {
      const policy = config[name][kind]
      const label = `${name}.${kind}`
      keys(policy, ['growth', 'roundBytes'], `config.${label}`)
      const growth = Number.isFinite(policy.growth) && policy.growth >= 0
      requireInput(growth, `invalid ${label} growth`)
      requireInput(bytes(policy.roundBytes), `invalid ${label} roundBytes`)
    }
  }
  return config
}

export function validateMetadata(metadata, directories) {
  keys(metadata, ['jobId', 'buildCommand', 'base', 'candidate'], 'metadata')
  const provenance = text(metadata.jobId) && text(metadata.buildCommand)
  requireInput(provenance, 'missing job/build provenance')
  for (const side of ['base', 'candidate']) {
    const source = metadata[side]
    keys(
      source,
      ['commit', 'tree', 'packageDir', 'wasmBindgenVersion', 'tools'],
      `metadata.${side}`,
    )
    for (const id of ['commit', 'tree']) {
      const valid =
        typeof source[id] === 'string' && /^[a-f0-9]{40}$/.test(source[id])
      requireInput(valid, `invalid ${side}.${id}`)
    }
    const path = source.packageDir
    const matches =
      text(path) && realpathSync(path) === realpathSync(directories[side])
    requireInput(matches, `${side} packageDir does not match CLI`)
    keys(source.tools, tools, `${side}.tools`)
    for (const tool of tools) {
      requireInput(text(source.tools[tool]), `missing ${side} tool ${tool}`)
      if (
        tool === 'wasmBindgen' &&
        source.wasmBindgenVersion !== metadata.base.wasmBindgenVersion
      )
        continue
      const matches = source.tools[tool] === metadata.base.tools[tool]
      requireInput(matches, `tool mismatch: ${tool}`)
    }
    const version = source.wasmBindgenVersion
    requireInput(
      typeof version === 'string' && wasmBindgenVersionPattern.test(version),
      `invalid ${side}.wasmBindgenVersion`,
    )
    requireInput(
      source.tools.wasmBindgen.split(/\r?\n/, 1)[0] ===
        `wasm-bindgen ${version}`,
      `tool mismatch: ${side} wasmBindgen does not match committed lock version`,
    )
  }
  const distinct =
    realpathSync(directories.base) !== realpathSync(directories.candidate)
  requireInput(distinct, 'base and candidate package directories must differ')
  return metadata
}

export function measure(directory) {
  return Object.fromEntries(
    Object.entries(artifacts).map(([name, file]) => {
      const path = join(realpathSync(directory), file)
      const content = readFileSync(path)
      requireInput(bytes(content.length), `empty artifact: ${path}`)
      const sizes = {
        file: path,
        sha256: createHash('sha256').update(content).digest('hex'),
        raw: content.length,
        gzip: gzipSync(content, { level: 9 }).length,
      }
      return [name, sizes]
    }),
  )
}

export function compare(base, candidate, config) {
  validateConfig(config)
  const metrics = []
  for (const name of Object.keys(artifacts)) {
    for (const kind of ['raw', 'gzip']) {
      const before = base?.[name]?.[kind]
      const after = candidate?.[name]?.[kind]
      requireInput(
        bytes(before) && bytes(after),
        `invalid measurement: ${name}.${kind}`,
      )
      const { growth, roundBytes } = config[name][kind]
      const limit = Math.ceil((before * (1 + growth)) / roundBytes) * roundBytes
      requireInput(bytes(limit), `unsafe limit: ${name}.${kind}`)
      metrics.push({
        artifact: name,
        kind,
        baseBytes: before,
        candidateBytes: after,
        limitBytes: limit,
        deltaBytes: after - before,
        deltaPercent: ((after - before) / before) * 100,
        growth,
        roundBytes,
        passed: after <= limit,
      })
    }
  }
  return { passed: metrics.every((metric) => metric.passed), metrics }
}

export function summary(report) {
  const { metadata, measurements, config, measurement } = report
  const evidence = { metadata, measurements, config, measurement }
  const lines = [
    `## WASM size: ${report.passed ? 'PASS' : 'FAIL'}`,
    '',
    '| Metric | Base bytes | Candidate bytes | Limit bytes | Delta bytes | Delta % | Result |',
    '| --- | ---: | ---: | ---: | ---: | ---: | --- |',
    ...report.metrics.map(
      (row) =>
        `| ${row.artifact}/${row.kind} | ${row.baseBytes} | ${row.candidateBytes} | ${row.limitBytes} | ${row.deltaBytes} | ${row.deltaPercent.toFixed(3)} | ${row.passed ? 'PASS' : 'FAIL'} |`,
    ),
    '',
    'Limits: ceil(baseBytes * (1 + growth) / roundBytes) * roundBytes; policies recorded below.',
    '',
    '```json',
    JSON.stringify(evidence, null, 2),
    '```',
    '',
  ]
  return lines.join('\n')
}

export function cli(args, environment = process.env) {
  const names = [
    'base-pkg',
    'candidate-pkg',
    'metadata',
    'config',
    'report',
    'summary',
  ]
  const options = Object.fromEntries(
    names.map((name) => [name, { type: 'string' }]),
  )
  const { values } = parseArgs({ args, options })
  for (const flag of ['base-pkg', 'candidate-pkg', 'metadata', 'report'])
    requireInput(text(values[flag]), `required --${flag}`)
  const summaryFile = values.summary ?? environment.GITHUB_STEP_SUMMARY
  requireInput(text(summaryFile), 'required --summary or GITHUB_STEP_SUMMARY')
  const defaultConfig = new URL(
    './bindings/devup-ui-wasm/size-budget.json',
    import.meta.url,
  )
  const configFile = values.config ?? defaultConfig
  const paths = [values.report, summaryFile].map((path) => resolve(path))
  const directories = {
    base: values['base-pkg'],
    candidate: values['candidate-pkg'],
  }
  // Reject outputs that could overwrite evidence or policy before writing anything.
  const files = Object.values(directories).flatMap((dir) =>
    Object.values(artifacts).map((file) => join(dir, file)),
  )
  const inputs = [values.metadata, configFile, ...files]
  const safe = inputs.every((file) => !paths.includes(realpathSync(file)))
  requireInput(
    paths[0] !== paths[1] && safe,
    'outputs must not overwrite inputs or each other',
  )
  const config = validateConfig(readJson(configFile))
  const metadata = validateMetadata(readJson(values.metadata), directories)
  const base = measure(directories.base)
  const candidate = measure(directories.candidate)
  const report = {
    schemaVersion: 1,
    ...compare(base, candidate, config),
    metadata,
    measurements: { base, candidate },
    config,
    measurement: {
      implementation: 'node:zlib.gzipSync',
      level: 9,
      node: process.version,
      zlib: process.versions.zlib,
    },
  }
  const markdown = summary(report)
  writeFileSync(values.report, JSON.stringify(report, null, 2) + '\n')
  if (values.summary) writeFileSync(summaryFile, markdown)
  else appendFileSync(summaryFile, markdown)
  console.info(markdown)
  return report.passed ? 0 : 1
}
