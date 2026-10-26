import assert from 'node:assert/strict'
import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { build } from 'vite'

export async function assertNativeMdxRepair(config, description) {
  let emitted = ''
  let emittedCss = ''
  const watcher = await build({
    ...config,
    plugins: [
      ...config.plugins,
      {
        name: 'observe-repaired-output',
        generateBundle(_options, bundle) {
          emitted = Object.values(bundle)
            .filter((item) => item.type === 'chunk')
            .map((item) => item.code)
            .join('\n')
          emittedCss = Object.values(bundle)
            .filter(
              (item) => item.type === 'asset' && item.fileName.endsWith('.css'),
            )
            .map((item) => String(item.source))
            .join('\n')
        },
      },
    ],
    build: { ...config.build, watch: {} },
  })
  assert(!Array.isArray(watcher) && 'on' in watcher)
  const events = []
  let notify
  watcher.on('event', (event) => {
    if (!['ERROR', 'BUNDLE_END'].includes(event.code)) return
    events.push(event)
    notify?.()
  })
  async function next() {
    let timer
    try {
      if (!events.length)
        await new Promise((resolve, reject) => {
          notify = resolve
          timer = setTimeout(
            () => reject(new Error('Native opaque MDX completion deadline')),
            30000,
          )
        })
      return events.shift()
    } finally {
      clearTimeout(timer)
      notify = undefined
    }
  }
  function history() {
    const path = join(config.root, 'df/numbering')
    return Object.fromEntries(
      readdirSync(path).map((name) => [
        name,
        readFileSync(join(path, name), 'utf8'),
      ]),
    )
  }
  try {
    assert.equal((await next()).code, 'BUNDLE_END')
    assert(emitted.includes('Prepared '))
    assert(!emitted.includes('Cannot run on the runtime'))
    const before = history()
    writeFileSync(description, '<Unclosed')
    const invalid = await next()
    assert.equal(invalid.code, 'ERROR')
    assert.match(invalid.error.message, /Unexpected end of file/)
    assert.deepEqual(history(), before)
    writeFileSync(description, 'Repaired **description**.')
    assert.equal((await next()).code, 'BUNDLE_END')
    assert(emitted.includes('Repaired '))
    assert(!emitted.includes('Cannot run on the runtime'))
    assert.match(emittedCss, /background:#b21329/i)
    assert.match(emittedCss, /color:#274869/i)
  } finally {
    await watcher.close()
  }
}
