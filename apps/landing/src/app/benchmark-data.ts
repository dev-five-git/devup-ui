import live from '../../../../benchmark-results.json'
import fixture from '../../../../e2e/fixtures/benchmark-results.json'

// The landing page shows the checked benchmark-results.json, which CI rewrites
// after every merge to main (benchmark-refresh.js), so the page shows the same
// run as the READMEs.
//
// The e2e builds set LANDING_BENCHMARK_DATA=fixture and render the frozen
// e2e/fixtures/benchmark-results.json instead, so a benchmark refresh never
// changes a screenshot baseline. Bench is a server component prerendered at
// build time, so this choice is made by the build and ships no client code.
export const BENCHMARK_SOURCE =
  process.env.LANDING_BENCHMARK_DATA === 'fixture' ? 'fixture' : 'live'

/**
 * What the landing page reads of a benchmark-results.json. Both files fit it:
 * the frozen fixture records no client bytes (`null`), while a live run does,
 * so neither file's inferred type can stand for the other.
 */
export interface BenchmarkResults {
  run: {
    id: string
    url: string
    commit: string
    next: string
    devupUi: string
    sha: string
  }
  samples: number
  rows: readonly {
    table: string
    target: string
    version: string
    median: number
    bytes: number
  }[]
}

export const benchmarkResults: BenchmarkResults =
  BENCHMARK_SOURCE === 'fixture' ? fixture : live
