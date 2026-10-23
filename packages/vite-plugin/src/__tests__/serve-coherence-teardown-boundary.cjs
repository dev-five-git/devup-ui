const assert = require('node:assert/strict')
const { fork } = require('node:child_process')
const { rmSync, writeFileSync } = require('node:fs')
const { join } = require('node:path')
const mode = process.argv[2]
const root = process.argv[3]
assert(root)
assert.equal(process.platform, 'win32')

async function main() {
  if (mode === 'holder') {
    process.chdir(root)
    process.on('message', () => {})
    assert(process.send)
    process.send('holding-directory')
    return
  }
  assert.equal(mode, 'owner')
  const holder = fork(__filename, ['holder', root], {
    stdio: ['ignore', 'inherit', 'inherit', 'ipc'],
  })
  /** @type {Promise<unknown>} */
  const ready = new Promise((resolve, reject) => {
    holder.once('error', reject)
    holder.once('message', resolve)
  })
  assert.equal(await ready, 'holding-directory')
  let denied = ''
  assert.throws(
    () => rmSync(root, { recursive: true }),
    (error) => {
      if (
        !(error instanceof Error) ||
        !('code' in error) ||
        typeof error.code !== 'string'
      )
        return false
      denied = error.code
      return ['EPERM', 'EBUSY', 'EACCES'].includes(denied)
    },
  )
  writeFileSync(join(root, 'boundary.json'), JSON.stringify({ denied, root }))
  // Exit the root while its real descendant is still owned by the native job.
  process.exit(0)
}
main().catch((error) => {
  console.error(error)
  process.exit(1)
})
