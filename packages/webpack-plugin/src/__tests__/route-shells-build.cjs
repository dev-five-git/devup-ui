// Builds a Next-like app (a root layout entry plus page entries) with real
// webpack and prints the stylesheets the plugin writes, as JSON.
const assert = require('node:assert/strict')
const {
  mkdirSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} = require('node:fs')
const { dirname, join } = require('node:path')
const {
  webpack,
} = require('../../../next-plugin/node_modules/next/dist/compiled/webpack/webpack')

console.info = (...args) => console.error(...args)

const [fixture, shells, layout] = process.argv.slice(2)
assert(fixture && ['on', 'off'].includes(shells))
mkdirSync(fixture, { recursive: true })
process.chdir(fixture)

function put(path, text) {
  const absolute = join(fixture, path)
  mkdirSync(dirname(absolute), { recursive: true })
  writeFileSync(absolute, text)
}

put('devup.json', '{}')
put(
  'src/components/Header.js',
  `import { css } from '@devup-ui/react';
export const header = css({ color: 'white', fontSize: [14, null, 18], _hover: { color: 'orange' } });`,
)
put(
  'src/components/Card.js',
  `import { css } from '@devup-ui/react';
export const card = css({ padding: [12, null, 20], _hover: { boxShadow: '0 2px 8px #0003' } });`,
)
put(
  'src/app/layout.js',
  `import { header } from '../components/Header.js';
export default function Layout() { return header }`,
)
put(
  'src/app/page.js',
  `import { css } from '@devup-ui/react'; import { card } from '../components/Card.js';
export default function Home() { return [card, css({ color: 'teal' })] }`,
)
put(
  'src/app/a/page.js',
  `import { css } from '@devup-ui/react'; import { card } from '../../components/Card.js';
export default function A() { return [card, css({ color: 'white', fontSize: 20 })] }`,
)
put(
  'src/app/b/page.js',
  `import { css } from '@devup-ui/react';
export default function B() { return css({ color: 'maroon', margin: [4, null, 12], _focus: { outline: '1px solid' } }) }`,
)

const { DevupUIWebpackPlugin } = require('../../dist/index.cjs')
const pages = [
  './src/app/page.js',
  './src/app/a/page.js',
  './src/app/b/page.js',
]
const compiler = webpack({
  mode: 'production',
  context: fixture,
  cache: false,
  devtool: false,
  entry: Object.fromEntries(
    [...(layout === 'none' ? [] : ['./src/app/layout.js']), ...pages].map(
      (entry, index) => [`e${index}`, entry],
    ),
  ),
  output: { path: join(fixture, 'out'), filename: '[name].js', clean: true },
  optimization: { minimize: false },
  experiments: { css: true },
  module: { rules: [{ test: /\.css$/, type: 'css' }] },
  plugins: [
    new DevupUIWebpackPlugin({
      engine: 'wasm',
      productionPrewarmFiles: [],
      routeShells: shells === 'on',
    }),
  ],
})

compiler.run((error, stats) => {
  if (error) throw error
  assert(stats && !stats.hasErrors(), stats?.toString())
  const cssDir = join(fixture, 'df', 'devup-ui')
  const sheets = Object.fromEntries(
    readdirSync(cssDir)
      .filter((name) => name.endsWith('.css'))
      .sort()
      .map((name) => [name, readFileSync(join(cssDir, name), 'utf-8')]),
  )
  compiler.close(() => {
    process.stdout.write(`${JSON.stringify(sheets)}\n`)
  })
})
