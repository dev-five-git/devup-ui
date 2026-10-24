const assert = require('node:assert/strict')
const { mkdirSync, realpathSync, symlinkSync } = require('node:fs')
const { dirname, join } = require('node:path')
const { put } = require('./reservation-native-fixture.cjs')
const { DevupUI } = require('../../dist/' + 'index.cjs')

const inputRoot = process.argv[2]
assert(inputRoot)
const root = realpathSync.native(inputRoot)
const source =
  "import {Box} from '@devup-ui/react'; export const A=<Box color='red'/>;"

async function main() {
  put(
    root,
    'package.json',
    '{"name":"cache-invalidation-query","type":"module"}',
  )
  put(root, 'devup.json', '{}')
  put(root, 'src/A.tsx', source)
  mkdirSync(join(root, 'node_modules'))
  symlinkSync(
    dirname(
      require.resolve('react/package.json', {
        paths: [join(__dirname, '../../../react')],
      }),
    ),
    join(root, 'node_modules/react'),
    'junction',
  )
  const { createServer } = await import('vite')
  const server = await createServer({
    root,
    configFile: false,
    server: { watch: null, fs: { allow: [root] } },
    logLevel: 'silent',
    plugins: [
      DevupUI({
        devupFile: join(root, 'devup.json'),
        distDir: join(root, 'df'),
        cssDir: join(root, 'df/css'),
        singleCss: true,
      }),
    ],
  })
  try {
    for (const query of ['', '?route=nested/value', '?route=/nested/value']) {
      const result = await server.transformRequest('/src/A.tsx' + query)
      assert(result)
      const cssUrl = /import\s*["']([^"']+\/devup-ui\.css\?[^"']+)["']/.exec(
        result.code,
      )?.[1]
      assert(cssUrl && cssUrl.startsWith('/df/css/devup-ui.css?'))
      const css = await server.transformRequest(cssUrl)
      assert(css)
      assert.match(css.code, /color:red/)
    }
  } finally {
    await server.close()
  }
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
