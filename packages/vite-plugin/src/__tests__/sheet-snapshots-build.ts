// Builds an ssr and then a client environment on one shared engine and prints
// the devup-ui CSS each environment wrote, by file name.
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

const root = process.argv[2] as string
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
    // In this order the server loads two sheets that are equal by then
    async buildApp(app) {
      await app.build(app.environments.ssr!)
      await app.build(app.environments.client!)
    },
  },
})
await builder.buildApp()
const css = (environment: string) => {
  const dir = join(root, 'dist', environment, 'assets')
  return Object.fromEntries(
    readdirSync(dir)
      .filter((file) => file.endsWith('.css'))
      .map((file) => [
        // Drop the content hash
        file.replace(/-[\w-]{8}\.css$/, '.css'),
        readFileSync(join(dir, file), 'utf8').replace(/\/\*!.*?\*\//, ''),
      ]),
  )
}
process.stdout.write(
  JSON.stringify({ client: css('client'), server: css('server') }),
)
