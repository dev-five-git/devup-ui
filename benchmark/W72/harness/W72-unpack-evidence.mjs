import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { createReadStream, mkdirSync, writeFileSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'
import { createInterface } from 'node:readline'
import { createGunzip } from 'node:zlib'

const [archive, output] = process.argv.slice(2)
assert.equal(typeof archive, 'string', 'Expected archive path')
assert.equal(typeof output, 'string', 'Expected fresh output directory')
const destination = resolve(output)
mkdirSync(destination, { recursive: true })
const lines = createInterface({
  input: createReadStream(archive).pipe(createGunzip()),
  crlfDelay: Infinity,
})
let count = 0
for await (const line of lines) {
  const record = JSON.parse(line)
  assert.equal(typeof record.file, 'string')
  assert.equal(basename(record.file), record.file)
  assert.match(record.file, /^W72-[A-Za-z0-9_.-]+\.(json|log)$/)
  assert.ok(record.encoding === 'utf8' || record.encoding === 'base64')
  assert.equal(typeof record.content, 'string')
  assert.match(record.sha256, /^[a-f0-9]{64}$/)
  const bytes = Buffer.from(record.content, record.encoding)
  const actual = createHash('sha256').update(bytes).digest('hex')
  assert.equal(actual, record.sha256, record.file)
  writeFileSync(join(destination, record.file), bytes, { flag: 'wx' })
  count++
}
process.stdout.write(
  JSON.stringify({ filesVerifiedAndExtracted: count }) + '\n',
)
