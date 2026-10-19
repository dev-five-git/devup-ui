import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { out } from './W72-corpus.mjs'

const lock = join(out, 'W72-real-build.lock')
const transaction = JSON.parse(readFileSync(lock, 'utf8'))
try {
  process.kill(transaction.pid, 0)
  throw new Error(
    'Recorded build process is still live; inspect before recovery',
  )
} catch (error) {
  if (!(error instanceof Error) || error.code !== 'ESRCH') throw error
}
if (existsSync(transaction.backup))
  writeFileSync(transaction.alias, readFileSync(transaction.backup))
else if (readFileSync(transaction.alias, 'utf8').includes('W72_REAL_ALIAS'))
  throw new Error('Alias installed without a durable backup')
const baseline = join(out, `${transaction.id}-generated-baseline.json`)
if (existsSync(baseline)) {
  for (const file of JSON.parse(readFileSync(baseline, 'utf8'))) {
    if (file.exists)
      writeFileSync(file.filename, Buffer.from(file.base64, 'base64'))
    else if (existsSync(file.filename)) rmSync(file.filename)
  }
}
rmSync(lock)
process.stdout.write(
  JSON.stringify({
    recovered: transaction.id,
    backupExisted: existsSync(transaction.backup),
  }) + '\n',
)
