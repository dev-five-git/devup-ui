import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { performance } from 'node:perf_hooks'

import {
  aliases,
  corpus,
  extract,
  hash,
  load,
  out,
  output,
  reset,
} from './W72-corpus.mjs'

const [variant, mode = 'identity', run = '0', shard = '0', shardCount = '1'] =
  process.argv.slice(2)
const start = performance.now()
const binding = await load(variant)
const importedMs = performance.now() - start
reset(binding)
const initializedMs = performance.now() - start
const data = corpus()
const result = {
  variant,
  mode,
  run,
  runtime: process.version,
  bun: process.versions.bun ?? null,
  importedMs,
  initializedMs,
}
const time = (action) => {
  const start = performance.now()
  const value = action()
  return { ms: performance.now() - start, value }
}
const errorText = (error) =>
  error instanceof Error ? error.message : String(error)

switch (mode) {
  case 'startup':
    result.firstExtractMs = time(() =>
      output(
        extract(
          binding,
          data.find((file) => file.group === 'synthetic'),
        ),
      ),
    ).ms
    break
  case 'identity': {
    const outputs = []
    for (const single of [true, false]) {
      for (const maps of ['map', 'skip']) {
        reset(binding)
        binding.seedFileMap(data.map((file) => file.filename))
        for (const file of data) {
          try {
            outputs.push({
              filename: file.filename,
              single,
              maps,
              ...output(extract(binding, file, maps, single)),
            })
          } catch (error) {
            outputs.push({
              filename: file.filename,
              single,
              maps,
              error: errorText(error),
            })
          }
        }
        outputs.push({
          single,
          maps,
          css: binding.getCss(null, false),
          sheet: binding.exportSheet(),
          classes: binding.exportClassMap(),
          files: binding.exportFileMap(),
          canonical: binding.exportCanonicalMap(),
        })
      }
    }
    const text = JSON.stringify(outputs)
    writeFileSync(join(out, `W72-output-${variant}-${run}.json`), text)
    result.sha256 = hash(text)
    result.errors = outputs.filter((row) => row.error)
    result.rows = outputs.length
    break
  }
  case 'timing': {
    const samples = []
    for (const maps of ['map', 'skip']) {
      for (const temperature of ['cold-state', 'warm']) {
        reset(binding)
        const valid = []
        for (const file of data) {
          try {
            output(extract(binding, file, maps))
            valid.push(file)
          } catch (error) {
            samples.push({
              filename: file.filename,
              maps,
              temperature,
              error: errorText(error),
            })
          }
        }
        for (let repetition = 0; repetition < 20; repetition++) {
          for (const file of valid) {
            if (temperature === 'cold-state') reset(binding)
            const sample = time(() => output(extract(binding, file, maps)))
            samples.push({
              group: file.group,
              filename: file.filename,
              maps,
              temperature,
              repetition,
              ms: sample.ms,
            })
          }
        }
      }
    }
    reset(binding)
    for (const file of data) {
      try {
        output(extract(binding, file))
      } catch (error) {
        result.statePreparationErrors ??= []
        result.statePreparationErrors.push(errorText(error))
      }
    }
    const state = {
      sheet: JSON.parse(binding.exportSheet()),
      classes: JSON.parse(binding.exportClassMap()),
      files: JSON.parse(binding.exportFileMap()),
    }
    const stateSamples = []
    for (let repetition = 0; repetition < 100; repetition++) {
      for (const [name, action] of [
        ['getCss', () => binding.getCss(null, false)],
        ['exportSheet', () => binding.exportSheet()],
        [
          'sheetRoundTrip',
          () => binding.importSheet(JSON.parse(binding.exportSheet())),
        ],
      ])
        stateSamples.push({ name, repetition, ms: time(action).ms })
    }
    const boundarySamples = []
    for (const bytes of [0, 1000, 100000, 1000000]) {
      const inert = 'x'.repeat(bytes)
      for (let repetition = 0; repetition < 100; repetition++)
        boundarySamples.push({
          bytes,
          ms: time(() =>
            binding.hasDevupUI('W72-inert.tsx', inert, '@devup-ui/react'),
          ).ms,
        })
    }
    result.samples = samples
    result.stateSamples = stateSamples
    result.boundarySamples = boundarySamples
    result.sheetBytes = Buffer.byteLength(JSON.stringify(state.sheet))
    break
  }
  case 'stack': {
    const depth = Number(run)
    const nested = {
      filename: 'W72-nesting.tsx',
      code: `import {Box} from '@devup-ui/react';export const x=${'<Box p={1}>'.repeat(depth)}text${'</Box>'.repeat(depth)};`,
    }
    result.depth = depth
    result.hash = hash(JSON.stringify(output(extract(binding, nested))))
    break
  }
  case 'headroom': {
    const selected = data.filter(
      (_, index) => index % Number(shardCount) === Number(shard),
    )
    const hashes = []
    const measured = time(() => {
      for (let repetition = 0; repetition < 3; repetition++) {
        for (const file of selected) {
          reset(binding)
          try {
            hashes.push([
              file.filename,
              hash(JSON.stringify(output(extract(binding, file, 'skip')))),
            ])
          } catch (error) {
            hashes.push([file.filename, errorText(error)])
          }
        }
      }
    })
    result.workMs = measured.ms
    result.hashes = hashes
    break
  }
  case 'reservation': {
    reset(binding)
    const rows = data.map((file) => [file.filename, file.code, 'W72-css'])
    result.reservationMs = time(() =>
      binding.reserveClassSlots(
        rows,
        '@devup-ui/react',
        true,
        false,
        false,
        aliases,
      ),
    ).ms
    result.count = binding.getUnreservedClassSlotCount()
    break
  }
  default:
    throw new Error(`Unknown mode ${mode}`)
}
result.wallMs = performance.now() - start
result.memory = process.memoryUsage()
result.resourceUsage = process.resourceUsage()
writeFileSync(
  join(out, `W72-${mode}-${variant}-${run}-${shard}.json`),
  JSON.stringify(result),
)
process.stdout.write(
  JSON.stringify({
    variant,
    mode,
    run,
    importedMs,
    initializedMs,
    wallMs: result.wallMs,
    maxRSS: result.resourceUsage.maxRSS,
    hash: result.sha256,
    errors: result.errors?.length,
  }) + '\n',
)
