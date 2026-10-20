const {
  mkdirSync,
  realpathSync,
  symlinkSync,
  writeFileSync,
} = require('node:fs')
const { createRequire } = require('node:module')
const { dirname, join, resolve } = require('node:path')

const repository = resolve(__dirname, '../../../..')
const fromNext = createRequire(
  join(repository, 'packages/next-plugin/package.json'),
)
const fromFramework = createRequire(fromNext.resolve('next/package.json'))
/** @param {string} root @param {string} name @param {string} contents */
function put(root, name, contents) {
  const file = join(root, name)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, contents)
}
/** @param {string} root @param {string} name @param {string} target */
function junction(root, name, target) {
  const destination = join(root, 'node_modules', name)
  mkdirSync(dirname(destination), { recursive: true })
  symlinkSync(
    realpathSync(target),
    destination,
    process.platform === 'win32' ? 'junction' : 'dir',
  )
}
/** @param {string} root */
function fixture(root) {
  for (const name of ['next', 'react', 'react-dom'])
    junction(root, name, dirname(fromFramework.resolve(`${name}/package.json`)))
  junction(root, '@devup-ui/react', join(repository, 'packages/react'))
  junction(
    root,
    '@devup-ui/next-plugin',
    join(repository, 'packages/next-plugin'),
  )
  put(
    root,
    'package.json',
    JSON.stringify({ name: 'native-next-reservation', private: true }),
  )
  put(root, 'devup.json', '{}')
  put(
    root,
    'next.config.js',
    `const observer = require(${JSON.stringify(join(__dirname, 'native-webpack-observer.cjs'))});
const { DevupUI } = require('@devup-ui/next-plugin');
module.exports = DevupUI({ webpack: observer.configure, experimental: { cpus: 1 } }, { singleCss: true });`,
  )
  put(
    root,
    'src/app/layout.js',
    `export default function Layout({ children }) { return <html><body>{children}</body></html> }`,
  )
  put(
    root,
    'src/app/client.js',
    `'use client';
import { Box, keyframes } from '@devup-ui/react';
const fade = keyframes({ from: { opacity: 0.25 }, to: { opacity: 1 } });
export default function Client({ color }) { return <Box id="client-proof" color={color} bg="navy" animationName={fade}>client-proof</Box> }`,
  )
  put(root, 'src/app/page.js', page(false))
  put(
    root,
    'src/app/edge/route.js',
    `import { css } from '@devup-ui/react';
export const runtime = 'edge';
const edge = css({ borderColor: 'goldenrod' });
export function GET() { return Response.json({ edge }); }`,
  )
}
/** @param {boolean} revised */
function page(revised) {
  return `import { Box } from '@devup-ui/react';
import Client from './client';
${revised ? "import A from './A'; import B from './B';" : ''}
export default function Page() { return <Box id="server-proof" bg="maroon" p="13px">server-proof<Client color="cyan" />${revised ? '<A /><B />' : ''}</Box> }`
}
/** @param {string} root */
function revision(root) {
  put(
    root,
    'src/app/B.js',
    `import { Box } from '@devup-ui/react'; export default function B() { return <Box id="new-b" bg="seagreen">new-b</Box> }`,
  )
  put(
    root,
    'src/app/A.js',
    `import { Box } from '@devup-ui/react'; export default function A() { return <Box id="new-a" bg="orchid">new-a</Box> }`,
  )
  put(root, 'src/app/page.js', page(true))
}
module.exports = { fixture, revision, repository, fromNext }
