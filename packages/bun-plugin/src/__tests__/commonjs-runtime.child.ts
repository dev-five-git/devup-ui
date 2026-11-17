// Loads every source kind through the registered runtime plugin, in a fresh
// Bun process without the repository's test preload. Prints what `require()`
// and `import` return, and the stylesheet the plugin wrote.
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { register } from '../plugin'

const [root, order, ...files] = process.argv.slice(2)
if (!root) throw new Error('Expected a fixture root')
const importFirst = order === 'import-first'
process.chdir(root)
await register({ engine: 'wasm' })

function describe(value: unknown): unknown {
  if (typeof value === 'function') return { function: value() }
  if (typeof value !== 'object' || value === null) return value
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, describe(Reflect.get(value, key))]),
  )
}

const require = createRequire(join(root, 'entry.js'))
const results: Record<string, unknown> = {}
for (const file of files) {
  const path = join(root, file)
  let imported: Record<string, unknown> = {}
  if (importFirst) imported = await import(path)
  const required: unknown = require(path)
  if (!importFirst) imported = await import(path)
  results[file] = {
    required: describe(required),
    imported: describe({ ...imported }),
    same: 'default' in imported && imported.default === required,
  }
}
process.stdout.write(
  JSON.stringify({
    results,
    css: readFileSync(join(root, 'df', 'devup-ui', 'devup-ui.css'), 'utf-8'),
  }),
)
