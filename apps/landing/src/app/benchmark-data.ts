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

export const benchmarkResults: typeof live =
  BENCHMARK_SOURCE === 'fixture' ? fixture : live
