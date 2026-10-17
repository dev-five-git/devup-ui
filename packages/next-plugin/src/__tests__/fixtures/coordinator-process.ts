import { join, resolve } from 'node:path'

import { startCoordinator } from '../../coordinator'
import { createWasm } from '../../wasm'

const [root = '', token = '', mode = ''] = process.argv.slice(2)

const handle = startCoordinator({
  wasm: createWasm(root),
  package: '@devup-ui/react',
  cssDir: join(root, 'df', 'devup-ui'),
  singleCss: false,
  importAliases: {},
  coordinatorPortFile: join(root, 'df', 'coordinator.port'),
  canonicalMap: {},
  projectRoot: root,
  identity: { project: resolve(root), token },
  watch: mode !== 'shutdown',
  expectedBaseFiles: mode === 'shutdown' ? ['src/missing.tsx'] : undefined,
})
await handle.ready
// The endpoint never keeps a process alive on its own
const keepAlive = setInterval(() => undefined, 1000)
if (mode === 'shutdown') {
  process.stdin.once('data', () => {
    handle.close()
    clearInterval(keepAlive)
    process.stdin.pause()
  })
}
console.info('ready')
