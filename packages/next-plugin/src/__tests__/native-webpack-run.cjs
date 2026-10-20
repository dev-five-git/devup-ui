const assert = require('node:assert/strict')
const { spawnSync } = require('node:child_process')
const {
  cpSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} = require('node:fs')
const { join } = require('node:path')
const { fixture, fromNext } = require('./native-webpack-fixture.cjs')
const {
  parseEvents,
  parseStateReceipt,
} = require('./native-webpack-receipt.ts')
const {
  assertReserved,
  assertRetained,
  assertPhysicalOrder,
} = require('./native-webpack-checks.ts')

const mode = process.argv[2]
assert(mode === 'production' || mode === 'development')
const evidence = process.env.DEVUP_NATIVE_EVIDENCE
const archive = evidence ? mkdtempSync(join(evidence, `${mode}-`)) : undefined
const root = realpathSync.native(
  mkdtempSync(
    join(realpathSync.native(join(__dirname, '../..')), '.native-next-'),
  ),
)
/** @type {NodeJS.ProcessEnv} */
const env = {
  ...Object.fromEntries(
    Object.entries(process.env).filter(
      ([key]) => !['TURBOPACK', 'CI', 'NODE_ENV'].includes(key),
    ),
  ),
  NEXT_TELEMETRY_DISABLED: '1',
  NODE_ENV: mode,
}
/** @param {readonly string[]} args */
function command(args) {
  const result = spawnSync(
    process.execPath,
    [join(__dirname, 'native-webpack-owned.cjs'), '180000', ...args],
    {
      cwd: root,
      env,
      encoding: 'utf8',
      maxBuffer: 32 * 1024 * 1024,
    },
  )
  process.stdout.write(result.stdout ?? '')
  process.stderr.write(result.stderr ?? '')
  if (archive) {
    const label = args.includes('build') ? 'build' : 'server'
    writeFileSync(join(archive, `${label}-stdout.txt`), result.stdout ?? '')
    writeFileSync(join(archive, `${label}-stderr.txt`), result.stderr ?? '')
  }
  assert.ifError(result.error)
  assert.equal(
    result.status,
    0,
    `native Next command failed: ${args.join(' ')}`,
  )
}
function saveEvidence() {
  const directory = archive
  if (!directory) return
  for (const name of ['.next', 'df', 'src', 'next.config.js', 'devup.json'])
    if (existsSync(join(root, name)))
      cpSync(join(root, name), join(directory, name), { recursive: true })
  for (const name of [
    'native-evidence.jsonl',
    `native-${mode}.json`,
    'native-cleanup.json',
  ]) {
    try {
      writeFileSync(
        join(directory, `${mode}-${name}`),
        readFileSync(join(root, name)),
      )
    } catch (error) {
      if (!(
        error instanceof Error &&
        'code' in error &&
        error.code === 'ENOENT'
      ))
        throw error
    }
  }
}
try {
  fixture(root)
  if (mode === 'production')
    command([fromNext.resolve('next/dist/bin/next'), 'build', '--webpack'])
  command([join(__dirname, 'native-webpack-server.cjs'), root, mode])
  const records = parseEvents(
    readFileSync(join(root, 'native-evidence.jsonl'), 'utf8'),
  )
  assertReserved(records)
  const reservations = records.filter((entry) => entry.kind === 'reserve')
  assert(reservations.length > 0)
  assert(
    reservations.every((entry) =>
      entry.args[0].some(([id]) => id === 'src/app/client.js'),
    ),
  )
  for (const entry of records.filter((entry) => entry.kind === 'extract'))
    assert.equal(entry.after, 0, `residual from ${entry.args[0]}`)
  for (const role of ['client', 'nodejs', 'edge']) {
    assert(
      records.some(
        (entry) =>
          entry.kind === 'options' &&
          entry.compilerRole === role &&
          entry.delegation === 1,
      ),
    )
    assert(
      records.some(
        (entry) => entry.kind === 'compilation' && entry.role === role,
      ),
    )
  }
  const receipt = parseStateReceipt(
    readFileSync(join(root, `native-${mode}.json`), 'utf8'),
  )
  if (mode === 'development') {
    assertRetained(receipt.old, receipt.final)
    const batch = reservations.find((entry) =>
      entry.args[0].some(([id]) => id === 'src/app/A.js'),
    )
    assert(batch)
    const ids = batch.args[0].map(([id]) => id)
    assert(ids.includes('src/app/B.js'))
    assertPhysicalOrder(batch.args[0], root)
  }
  console.info(
    JSON.stringify({
      mode,
      reservations: reservations.length,
      extracts: records.filter((entry) => entry.kind === 'extract').length,
      roles: [
        ...new Set(
          records
            .filter((entry) => entry.kind === 'compilation')
            .map((entry) => entry.role),
        ),
      ],
      residual: receipt.final.residual,
    }),
  )
} finally {
  try {
    saveEvidence()
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}
