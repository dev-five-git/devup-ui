import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { out } from './W72-corpus.mjs'

const rows = readdirSync(out)
  .filter((name) =>
    /^W72-real-(landing|vite)-(W1|W2|N1)-\d+-\d+-\d+-[\da-f-]+\.json$/.test(
      name,
    ),
  )
  .map((file) => ({
    file,
    ...JSON.parse(readFileSync(join(out, file), 'utf8')),
  }))
const latest = new Map()
const timingRounds = Number(process.env.W72_TIMING_ROUNDS ?? Infinity)
for (const row of rows
  .filter(
    (row) => row.status === 'success' && Number(row.repetition) < timingRounds,
  )
  .sort((a, b) => a.startedAt.localeCompare(b.startedAt)))
  latest.set(`${row.app}/${row.variant}/${row.repetition}`, row)
const selected = [...latest.values()]
const report = {
  verificationBuilds: rows
    .filter((row) => Number(row.repetition) >= timingRounds)
    .map((row) => ({
      file: row.file,
      status: row.status,
      aliasRestored: row.aliasRestoredByteIdentically,
    })),
  failed: rows
    .filter((row) => row.status !== 'success')
    .map((row) => ({
      file: row.file,
      error: row.error?.message,
      cleanup: row.cleanupError,
      collection: row.collectionError,
    })),
  samples: selected.map((row) => {
    const idle =
      sumTimes(row.environmentAfter, 'idle') -
      sumTimes(row.environmentBefore, 'idle')
    const total = ['user', 'nice', 'sys', 'idle', 'irq'].reduce(
      (sum, field) =>
        sum +
        (sumTimes(row.environmentAfter, field) -
          sumTimes(row.environmentBefore, field)),
      0,
    )
    const watchdog = JSON.parse(
      readFileSync(
        join(
          out,
          `W72-watch-real-${row.app}-${row.variant}-${row.repetition}.log.json`,
        ),
        'utf8',
      ),
    )
    return {
      app: row.app,
      variant: row.variant,
      repetition: row.repetition,
      file: row.file,
      wallMs: row.wallMs,
      insideMs: row.insideDevupMs,
      share: row.insideDevupShareOfBuildWall,
      observationMs: row.apiProcesses.reduce(
        (sum, process) => sum + process.observationMs,
        0,
      ),
      cpuBusy: 1 - idle / total,
      cpuLoadBefore: row.environmentBefore.windows.cpu[0].LoadPercentage,
      cpuLoadAfter: row.environmentAfter.windows.cpu[0].LoadPercentage,
      freeGiB: row.environmentBefore.freeBytes / 1024 ** 3,
      peakTreeRssMiB: watchdog.peakTreeRssBytes / 1024 ** 2,
      watchdogVersion: watchdog.watchdogVersion ?? 'legacy-watchdog',
      aliasRestored: row.aliasRestoredByteIdentically,
      cleaned: row.cleanedChildren,
      apis: row.apiCallCount,
    }
  }),
  identities: [],
}
function sumTimes(snapshot, field) {
  return snapshot.cpuTimes.reduce(
    (sum, cpu) => sum + (cpu.times[field] ?? 0),
    0,
  )
}
for (const app of ['landing', 'vite']) {
  const baseline = latest.get(`${app}/W1/0`)
  if (!baseline) continue
  const css = (row) =>
    row.devupCss
      .filter((file) => file.filename.startsWith(`apps/${app}/df/`))
      .sort((a, b) => a.filename.localeCompare(b.filename))
  const codeMaps = (row) => {
    const map = new Map()
    for (const proc of row.apiProcesses)
      for (const record of proc.observations) {
        const key = `${record.filename}|${record.inputSha256}`
        map.set(key, {
          code: record.code,
          map: record.map,
          cssFile: record.cssFile,
          dependencies: record.dependencies,
        })
      }
    return map
  }
  const expected = codeMaps(baseline)
  const finalPayloads = (row, extension) =>
    row.outputTrees
      .flatMap((tree) => tree.files)
      .filter(
        (file) =>
          file.filename.startsWith(`apps/${app}/dist/`) &&
          file.filename.endsWith(extension),
      )
      .map((file) => file.sha256)
      .sort()
  const expectedFinalCss = JSON.stringify(finalPayloads(baseline, '.css'))
  const expectedFinalJs = JSON.stringify(finalPayloads(baseline, '.js'))
  for (const row of selected.filter((row) => row.app === app)) {
    const actual = codeMaps(row)
    const differences = [
      ...new Set([...expected.keys(), ...actual.keys()]),
    ].filter(
      (key) =>
        JSON.stringify(expected.get(key)) !== JSON.stringify(actual.get(key)),
    )
    const fields = Object.fromEntries(
      ['code', 'map', 'cssFile', 'dependencies'].map((field) => [
        field,
        differences.filter(
          (key) => expected.get(key)?.[field] !== actual.get(key)?.[field],
        ).length,
      ]),
    )
    const expectedCss = new Map(
      css(baseline).map((file) => [file.filename, file.sha256]),
    )
    const cssDifferences = css(row).filter(
      (file) => expectedCss.get(file.filename) !== file.sha256,
    )
    report.identities.push({
      app,
      variant: row.variant,
      repetition: row.repetition,
      dfCssIdentical:
        JSON.stringify(css(baseline)) === JSON.stringify(css(row)),
      cssFiles: css(row).length,
      cssDifferences: cssDifferences.length,
      cssExamples: cssDifferences.slice(0, 3),
      transformedRecords: actual.size,
      codeMapDifferences: differences.length,
      finalCssFiles: finalPayloads(row, '.css').length,
      finalCssPayloadsIdentical:
        JSON.stringify(finalPayloads(row, '.css')) === expectedFinalCss,
      finalJsPayloadsIdentical:
        JSON.stringify(finalPayloads(row, '.js')) === expectedFinalJs,
      fields,
      examples: differences.slice(0, 5),
    })
  }
}
writeFileSync(
  join(out, 'W72-real-summary.json'),
  JSON.stringify(report, null, 2),
)
process.stdout.write(JSON.stringify(report, null, 2) + '\n')
