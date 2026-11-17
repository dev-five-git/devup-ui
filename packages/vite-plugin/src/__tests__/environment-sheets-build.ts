// Builds a client and an ssr environment on one shared engine, in the order
// given, and prints the CSS each environment wrote.
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join } from 'node:path'

import { createBuilder } from 'vite'

import { DevupUI } from '../plugin'

const [root, order, mode] = process.argv.slice(2) as [string, string, string]
rmSync(root, { recursive: true, force: true })
const put = (path: string, code: string) => {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, code)
}
const style = (name: string, color: string) =>
  `export const ${name} = css({ color: '${color}' })`
put('devup.json', '{}')
put('package.json', '{"type":"module"}')
put('index.html', '<script type="module" src="/src/client.js"></script>')
put(
  'src/client.js',
  `import { css } from '@devup-ui/react'\nimport { shared } from './shared.js'\n${style('client', 'red')}\nconsole.log(shared, client)`,
)
put(
  'src/shared.js',
  `import { css } from '@devup-ui/react'\n${style('shared', 'blue')}`,
)
// Only the server renders this module, so only the ssr build extracts it
put(
  'src/server.js',
  `import { css } from '@devup-ui/react'\nimport { shared } from './shared.js'\n${style('server', 'green')}\nexport const render = () => shared + server`,
)
process.chdir(root)
const builder = await createBuilder({
  configFile: false,
  root,
  logLevel: 'silent',
  plugins: [
    DevupUI({
      engine: 'wasm',
      distDir: join(root, 'df'),
      devupFile: join(root, 'devup.json'),
      singleCss: mode === 'single',
    }),
  ],
  environments: {
    client: {
      build: {
        outDir: 'dist/client',
        rollupOptions: { input: join(root, 'index.html') },
      },
    },
    ssr: {
      build: {
        outDir: 'dist/server',
        ssr: true,
        emitAssets: true,
        rollupOptions: { input: join(root, 'src/server.js') },
      },
    },
  },
  builder: {
    async buildApp(app) {
      for (const name of order.split(','))
        await app.build(app.environments[name]!)
    },
  },
})
await builder.buildApp()
const css = (environment: string) => {
  const dir = join(root, 'dist', environment, 'assets')
  return readdirSync(dir)
    .filter((file) => file.endsWith('.css'))
    .map((file) => readFileSync(join(dir, file), 'utf8'))
    .sort()
}
process.stdout.write(
  JSON.stringify({ client: css('client'), server: css('server') }),
)
