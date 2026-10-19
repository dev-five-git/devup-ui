import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { hash, out } from './W72-corpus.mjs'
const observations = []
for (let repetition = 0; repetition < 6; repetition++) {
  const read = (workers, shard) =>
    JSON.parse(
      readFileSync(
        join(out, `W72-headroom-N1-${repetition}-${workers}-${shard}.json`),
        'utf8',
      ),
    ).hashes
  const normalize = (values) =>
    values.map((value) => JSON.stringify(value)).sort()
  const serial = normalize(read(1, 0))
  const parallel = normalize([...read(2, 0), ...read(2, 1)])
  assert.deepEqual(parallel, serial)
  observations.push({
    repetition,
    candidates: serial.length,
    identical: true,
    sha256: hash(JSON.stringify(serial)),
  })
}
writeFileSync(
  join(out, 'W72-headroom-identity.json'),
  JSON.stringify(observations, null, 2),
)
process.stdout.write(JSON.stringify(observations) + '\n')
