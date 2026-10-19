import { execFileSync, spawn } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { performance } from 'node:perf_hooks'

import { corpus, hash, out, root } from './W72-corpus.mjs'

const mode = process.argv[2] ?? 'identity'
const variants = (process.env.W72_VARIANTS ?? 'W1,W2,N1').split(',')
const runtime = process.env.W72_RUNTIME ?? join(out, 'W72-node/node.exe')
const env = { ...process.env, W72_ROOT: root, W72_OUT: out }
delete env.CI
const script = join(out, 'W72-measure.mjs')
const run = (variant, mode, repetition, extra = []) =>
  execFileSync(runtime, [script, variant, mode, String(repetition), ...extra], {
    cwd: root,
    env,
    stdio: 'inherit',
  })
corpus()
switch (mode) {
  case 'api': {
    for (const variant of variants)
      execFileSync(runtime, [join(out, 'W72-api.mjs'), variant], {
        cwd: root,
        env,
        stdio: 'inherit',
      })
    const baseline = readFileSync(join(out, 'W72-api-W1.json'), 'utf8')
    process.stdout.write(
      JSON.stringify(
        variants.map((variant) => ({
          variant,
          identity:
            readFileSync(join(out, `W72-api-${variant}.json`), 'utf8') ===
            baseline,
        })),
      ) + '\n',
    )
    break
  }
  case 'identity': {
    for (let repetition = 0; repetition < 3; repetition++) {
      for (const variant of repetition % 2 ? variants.toReversed() : variants)
        run(variant, mode, repetition)
    }
    const baseline = JSON.parse(
      readFileSync(join(out, 'W72-output-W1-0.json'), 'utf8'),
    )
    const comparisons = []
    for (const variant of variants) {
      for (let repetition = 0; repetition < 3; repetition++) {
        const rows = JSON.parse(
          readFileSync(
            join(out, `W72-output-${variant}-${repetition}.json`),
            'utf8',
          ),
        )
        const differences = []
        for (let index = 0; index < baseline.length; index++) {
          const expected = baseline[index]
          const actual = rows[index]
          for (const key of new Set([
            ...Object.keys(expected),
            ...Object.keys(actual),
          ])) {
            if (JSON.stringify(expected[key]) !== JSON.stringify(actual[key]))
              differences.push({
                index,
                filename: expected.filename ?? 'state',
                single: expected.single,
                maps: expected.maps,
                key,
                expectedSha: hash(JSON.stringify(expected[key]) ?? 'undefined'),
                actualSha: hash(JSON.stringify(actual[key]) ?? 'undefined'),
              })
          }
        }
        comparisons.push({ variant, repetition, differences })
      }
    }
    writeFileSync(
      join(out, 'W72-identity-summary.json'),
      JSON.stringify(comparisons, null, 2),
    )
    process.stdout.write(
      JSON.stringify(
        comparisons.map(({ differences, ...row }) => ({
          ...row,
          differences: differences.length,
          fields: [...new Set(differences.map((item) => item.key))],
        })),
        null,
        2,
      ) + '\n',
    )
    break
  }
  case 'startup':
  case 'timing':
  case 'reservation': {
    const count = mode === 'startup' ? 30 : mode === 'timing' ? 3 : 10
    for (let repetition = 0; repetition < count; repetition++) {
      for (const variant of repetition % 2 ? variants.toReversed() : variants)
        run(variant, mode, repetition)
    }
    break
  }
  case 'stack': {
    const observations = []
    for (const depth of [16, 32, 64, 128, 256, 512]) {
      for (const variant of variants) {
        try {
          run(variant, mode, depth)
          observations.push({ depth, variant, passed: true })
        } catch (error) {
          if (!(error instanceof Error)) throw error
          observations.push({
            depth,
            variant,
            passed: false,
            status: error.status ?? null,
            signal: error.signal ?? null,
            message: error.message,
          })
        }
      }
    }
    writeFileSync(
      join(out, 'W72-stack-summary.json'),
      JSON.stringify(observations, null, 2),
    )
    break
  }
  case 'headroom': {
    const observations = []
    for (let repetition = 0; repetition < 6; repetition++) {
      for (const concurrency of repetition % 2 ? [2, 1] : [1, 2]) {
        const start = performance.now()
        await Promise.all(
          Array.from(
            { length: concurrency },
            (_, shard) =>
              new Promise((resolve, reject) => {
                const child = spawn(
                  runtime,
                  [
                    script,
                    'N1',
                    mode,
                    `${repetition}-${concurrency}`,
                    String(shard),
                    String(concurrency),
                  ],
                  { cwd: root, env, stdio: 'inherit' },
                )
                child.once('error', reject)
                child.once('exit', (code, signal) =>
                  code === 0
                    ? resolve()
                    : reject(new Error(`Headroom child: ${code}/${signal}`)),
                )
              }),
          ),
        )
        observations.push({
          repetition,
          concurrency,
          wallMs: performance.now() - start,
        })
      }
    }
    writeFileSync(
      join(out, 'W72-headroom-summary.json'),
      JSON.stringify(observations, null, 2),
    )
    break
  }
  default:
    throw new Error(`Unknown run mode ${mode}`)
}
