import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { out } from './W72-corpus.mjs'

const read = (name) => JSON.parse(readFileSync(join(out, name), 'utf8'))
const micro = read('W72-summary.json')
const real = read('W72-real-summary.json')
const fmt = (value) => value.toFixed(3)
const pair = (row) => `${fmt(row.median)} / ${fmt(row.p95)}`
const stats = (values) => {
  const sorted = [...values].sort((a, b) => a - b)
  const mean = values.reduce((a, b) => a + b, 0) / values.length
  return {
    median: sorted[Math.ceil(sorted.length / 2) - 1],
    min: sorted[0],
    max: sorted.at(-1),
    cv:
      Math.sqrt(
        values.reduce((sum, x) => sum + (x - mean) ** 2, 0) / values.length,
      ) / mean,
  }
}
let text =
  '## Extraction And State Calls\n\nAll latency pairs are median / p95 in milliseconds; nearest-rank percentiles. Extraction columns select the source-map-enabled path; no-map results remain in the raw summary. Corpus sums include both successful fixture inputs and the 96 precompiled MDX inputs, not Markdown compilation or the six imported-context negative fixtures.\n\n| Variant | Corpus Cold State | Corpus Warm | 5000 Elements Warm | getCss | Sheet Export | Sheet Round Trip |\n|---|---:|---:|---:|---:|---:|---:|\n'
for (const variant of ['W1', 'W2', 'N1', 'Nz']) {
  const find = (rows, fn) =>
    rows.find((row) => row.variant === variant && fn(row))
  text += `| ${variant} | ${pair(find(micro.corpusPasses, (r) => r.maps === 'map' && r.temperature === 'cold-state'))} | ${pair(find(micro.corpusPasses, (r) => r.maps === 'map' && r.temperature === 'warm'))} | ${pair(find(micro.timing, (r) => r.group === 'synthetic' && r.maps === 'map' && r.temperature === 'warm'))} | ${pair(find(micro.state, (r) => r.name === 'getCss'))} | ${pair(find(micro.state, (r) => r.name === 'exportSheet'))} | ${pair(find(micro.state, (r) => r.name === 'sheetRoundTrip'))} |\n`
}
text +=
  '\n## Fresh Process Startup\n\n30 interleaved fresh processes per variant, filesystem caches left intact. Import includes require/load; initialization adds reset, debug/theme/shorthand registration. First extraction is the synthetic 5000-element file including getters/free. RSS is the maximum pooled across 30 startup plus 3 timing processes per variant, not startup alone. Bytes count only the raw WASM/native binary, excluding loader/glue and compressed/npm distribution costs.\n\n| Variant | Import Median / p95 ms | Import + Init Median / p95 ms | First Extract Median / p95 ms | Max Pooled RSS MiB | Binary Bytes |\n|---|---:|---:|---:|---:|---:|\n'
for (const variant of ['W1', 'W2', 'N1', 'Nz']) {
  const item = (field) =>
    micro.startup.find((row) => row.variant === variant && row.field === field)
  text += `| ${variant} | ${pair(item('importedMs'))} | ${pair(item('initializedMs'))} | ${pair(item('firstExtractMs'))} | ${fmt(micro.rss.find((row) => row.variant === variant).max)} | ${micro.sizes.find((row) => row.variant === variant).bytes.toLocaleString('en-US')} |\n`
}
text +=
  '\n## Production Builds\n\nAlternating cold builds per app/variant; each odd round reverses W1-W2-N1. Individual samples are retained rather than dropping slow runs. Wall time includes the landing generation step and actual CLI, not inventory/preflight/cleanup. Inside Devup is summed outermost synchronous binding API time, excluding separately measured observation work and binding import.\n\n| App | Variant | Individual Wall ms | Median Wall ms | CV | Median Inside ms | Median Inside Share | Max Tree RSS MiB |\n|---|---|---|---:|---:|---:|---:|---:|\n'
for (const app of ['landing', 'vite'])
  for (const variant of ['W1', 'W2', 'N1']) {
    const rows = real.samples
      .filter((row) => row.app === app && row.variant === variant)
      .sort((a, b) => Number(a.repetition) - Number(b.repetition))
    const stat = stats(rows.map((row) => row.wallMs))
    const measuredRss = rows.filter(
      (row) => row.watchdogVersion === 'current-tree-v2-root-included',
    )
    text += `| ${app} | ${variant} | ${rows.map((row) => fmt(row.wallMs)).join(', ')} | ${fmt(stat.median)} | ${(stat.cv * 100).toFixed(1)}% | ${fmt(stats(rows.map((row) => row.insideMs)).median)} | ${(stats(rows.map((row) => row.share)).median * 100).toFixed(2)}% | ${fmt(Math.max(...measuredRss.map((row) => row.peakTreeRssMiB)))} |\n`
  }
text +=
  '\nPeak tree RSS uses the root-inclusive v2 watchdog samples (at least three per app/variant), after correcting Windows creation-time precision and stale-parent PID attribution. Earlier timing samples are retained, but their older sampler RSS is not used for this column.\n'
const load = real.samples.flatMap((row) => [
  row.cpuLoadBefore,
  row.cpuLoadAfter,
])
text += `\nShared-machine Windows CPU-load snapshots range ${Math.min(...load)}-${Math.max(...load)}%; starting free memory ${Math.min(...real.samples.map((row) => row.freeGiB)).toFixed(2)}-${Math.max(...real.samples.map((row) => row.freeGiB)).toFixed(2)} GiB. Instrumentation getter/hash overhead ranges ${Math.min(...real.samples.map((row) => row.observationMs)).toFixed(3)}-${Math.max(...real.samples.map((row) => row.observationMs)).toFixed(3)} ms per build.\n`
text +=
  '\n## Production Identity Gate\n\nEach range is the number of differing diagnostic records/files against the first W1 build, including the baseline zero. Per-record keys do not distinguish repeated environment/options and are not alone a final-output proof. The last two columns independently compare multisets of actual dist CSS/JS payload hashes, ignoring filename-only renaming; they confirm landing differs even W1-versus-W1. The Vite demo is stable.\n\n| App | Variant | DF CSS Different | Code Records Different | Maps Different | CSS Filename Different | Dependencies Different | Final CSS Equal Runs | Final JS Equal Runs |\n|---|---|---:|---:|---:|---:|---:|---:|---:|\n'
const range = (values) => `${Math.min(...values)}-${Math.max(...values)}`
for (const app of ['landing', 'vite'])
  for (const variant of ['W1', 'W2', 'N1']) {
    const rows = real.identities.filter(
      (row) => row.app === app && row.variant === variant,
    )
    text += `| ${app} | ${variant} | ${range(rows.map((row) => row.cssDifferences))} | ${range(rows.map((row) => row.fields.code))} | ${range(rows.map((row) => row.fields.map))} | ${range(rows.map((row) => row.fields.cssFile))} | ${range(rows.map((row) => row.fields.dependencies))} | ${rows.filter((row) => row.finalCssPayloadsIdentical).length}/${rows.length} | ${rows.filter((row) => row.finalJsPayloadsIdentical).length}/${rows.length} |\n`
  }
text +=
  '\n## Boundary Proxy And Reservation\n\n`hasDevupUI` on a 1,000,000-byte ASCII non-import string measures inbound string conversion plus the real fast-negative scanner, not pure transfer. Native includes its actual N-API entry, WASM its actual generated glue. Reservation is the real no-callback sequential pre-pass over all physical corpus rows.\n\n| Variant | 1 MB Negative Scan Median / p95 ms | Reservation Median / p95 ms |\n|---|---:|---:|\n'
for (const variant of ['W1', 'W2', 'N1', 'Nz'])
  text += `| ${variant} | ${pair(micro.boundary.find((row) => row.variant === variant && row.bytes === 1000000))} | ${pair(micro.reservation.find((row) => row.variant === variant))} |\n`
const head = read('W72-headroom-summary.json')
text +=
  '\n## Isolated Process Headroom\n\nThis is not an async binding or a usable parallel reservation implementation: per-input state is reset and no-callback/no-map extraction plus output materialization is split across one or two isolated processes. A production plan still needs the authoritative sequential reservation and ordered sheet commit.\n\n| Processes | Individual Wall ms | Median Wall ms |\n|---|---|---:|\n'
for (const concurrency of [1, 2]) {
  const values = head
    .filter((row) => row.concurrency === concurrency)
    .map((row) => row.wallMs)
  text += `| ${concurrency} | ${values.map(fmt).join(', ')} | ${fmt(stats(values).median)} |\n`
}
writeFileSync(join(out, 'W72-numbers.md'), text)
process.stdout.write(text + '\n')
