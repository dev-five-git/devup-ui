import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { out } from './W72-corpus.mjs'
const read = (name) => JSON.parse(readFileSync(join(out, name), 'utf8'))
const comparisons = read('W72-identity-summary.json')
assert.equal(comparisons.length, 12)
for (const row of comparisons) {
  if (row.variant.startsWith('W')) assert.equal(row.differences.length, 0)
  else
    assert.ok(
      row.differences.every((difference) =>
        ['sheet', 'classes'].includes(difference.key),
      ),
    )
}
for (const variant of ['N1', 'Nz']) {
  const baseline = readFileSync(
    join(out, `W72-output-${variant}-0.json`),
    'utf8',
  )
  for (const run of [1, 2])
    assert.equal(
      readFileSync(join(out, `W72-output-${variant}-${run}.json`), 'utf8'),
      baseline,
    )
}
assert.ok(read('W72-stack-summary.json').every((row) => row.passed))
assert.ok(read('W72-headroom-identity.json').every((row) => row.identical))
const builds = read('W72-real-summary.json')
assert.ok(builds.samples.length >= 18)
assert.ok(builds.samples.every((row) => row.aliasRestored && row.freeGiB >= 7))
const observation = {
  corpusInputs: read('W72-corpus-manifest.json').length,
  comparisons: comparisons.length,
  nativeRepeatIdentity: true,
  stackCases: read('W72-stack-summary.json').length,
  realBuilds: builds.samples.length,
  realCssIdentity: builds.identities.every((row) => row.dfCssIdentical),
  realCodeMapIdentity: builds.identities.every(
    (row) => row.codeMapDifferences === 0,
  ),
  realDifferences: builds.identities.filter(
    (row) => !row.dfCssIdentical || row.codeMapDifferences > 0,
  ),
}
writeFileSync(
  join(out, 'W72-evidence-check.json'),
  JSON.stringify(observation, null, 2),
)
process.stdout.write(JSON.stringify(observation, null, 2) + '\n')
