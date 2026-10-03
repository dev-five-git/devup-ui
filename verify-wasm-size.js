// Tracks the size of the published WASM package and fails when it grows beyond
// the budget checked into bindings/devup-ui-wasm/size-budget.json.
//
//   bun verify-wasm-size.js                    measure pkg/, write wasm-size.json, fail above the budget
//   bun verify-wasm-size.js --write            set the budget to the measurement plus 1%, rounded up to 1 KiB
//   bun verify-wasm-size.js --write --from F   the same from a measurement file F (a CI run's `wasm-size` artifact)
//
// The budget is a ceiling, not a target: a pull request that grows the binary
// beyond it raises the budget in the same pull request, where a reviewer sees
// the size change next to the code that caused it.
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { gzipSync } from 'node:zlib'

const PACKAGE = 'bindings/devup-ui-wasm'
const BUDGET = join(PACKAGE, 'size-budget.json')
const HEADROOM = 1.01

const read = (file) => JSON.parse(readFileSync(file, 'utf8'))

function measure() {
  const sizes = {}
  for (const [name, file] of [
    ['wasm', 'index_bg.wasm'],
    ['glue', 'index.js'],
  ]) {
    const content = readFileSync(join(PACKAGE, 'pkg', file))
    sizes[name] = {
      raw: content.length,
      gzip: gzipSync(content, { level: 9 }).length,
    }
  }
  return sizes
}

const roundUp = (value) => Math.ceil((value * HEADROOM) / 1024) * 1024

const from = process.argv.indexOf('--from')
const measured = from === -1 ? measure() : read(process.argv[from + 1])

if (process.argv.includes('--write')) {
  const budget = {}
  for (const [name, size] of Object.entries(measured))
    budget[name] = { raw: roundUp(size.raw), gzip: roundUp(size.gzip) }
  writeFileSync(BUDGET, JSON.stringify(budget, null, 2) + '\n')
  console.info(`${BUDGET} rewritten`)
} else {
  writeFileSync('wasm-size.json', JSON.stringify(measured, null, 2) + '\n')
  const budget = read(BUDGET)
  const failures = []
  for (const [name, size] of Object.entries(measured)) {
    for (const kind of ['raw', 'gzip']) {
      const limit = budget[name][kind]
      const share = ((size[kind] / limit) * 100).toFixed(1)
      console.info(
        `${name} ${kind}: ${size[kind].toLocaleString()} bytes of the ${limit.toLocaleString()} budget (${share}%)`,
      )
      if (size[kind] > limit)
        failures.push(
          `${name} ${kind} is ${size[kind].toLocaleString()} bytes, ${(size[kind] - limit).toLocaleString()} over the budget of ${limit.toLocaleString()}`,
        )
    }
  }
  if (failures.length > 0) {
    console.error(
      `verify-wasm-size: the package grew beyond its budget:\n  - ${failures.join('\n  - ')}\nIf the growth is intended, raise the budget with \`bun verify-wasm-size.js --write\` in the same pull request.`,
    )
    process.exit(1)
  }
  console.info('verify-wasm-size: within the budget')
}
