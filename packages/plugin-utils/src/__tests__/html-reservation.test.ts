import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { htmlReservationEntries } from '../html-reservation'
import { collectReservationInputs } from '../reservation-discovery'
import { fixture, STYLE } from './reservation-fixture'

it.each([
  ['/&copy;A.js', '/©A.js'],
  ['/&#169;B.js', '/©B.js'],
  ['/&#xA9;C.js', '/©C.js'],
  ['/a&amp;b.js', '/a&b.js'],
  ['/a&notit;.js', '/a&notit;.js'],
])(
  'decodes native HTML attributes when the URL is %s',
  async (src, expected) => {
    // Given
    using fs = fixture()
    // When
    const result = await htmlReservationEntries(
      `<SCRIPT TYPE=module SRC="${src}"></SCRIPT>`,
      {
        command: 'build',
        root: fs.root,
        filename: join(fs.root, 'index.html'),
        publicDir: false,
      },
    )
    // Then
    expect(result).toEqual([
      { request: resolve(fs.root, `.${expected}`), context: fs.root },
    ])
  },
)

it.each([
  ['serve', false, '/nested/physical/%C2%A9%20A.js', '/physical/© A.js'],
  [
    'build',
    true,
    '/nested/physical/%C2%A9%20A.js',
    '/nested/physical/%C2%A9%20A.js',
  ],
  ['serve', false, '/physical/bad%URL.js', '/physical/bad%URL.js'],
] as const)(
  'projects native %s requests independently of substituted DEV=%s',
  async (command, dev, src, expected) => {
    // Given
    using fs = fixture()
    // When
    const entries = await htmlReservationEntries(
      `<script type=module src="${src}"></script>`,
      {
        command,
        base: '/nested/',
        root: fs.root,
        filename: join(fs.root, 'index.html'),
        publicDir: false,
        env: { DEV: dev, BASE_URL: '/nested/' },
      },
    )
    // Then
    expect(entries.map(({ request }) => request)).toEqual([
      resolve(fs.root, `.${expected}`),
    ])
  },
)

it('follows complete-document parsing contexts and first duplicate attributes', async () => {
  // Given
  using fs = fixture()
  const script =
    '<script type=module src="./real.js" src="./wrong.js"></script>'
  const html = `<!doctype html><head><noscript>${script}</noscript></head><body>
    <!-- ${script} --><style>${script}</style><textarea>${script}</textarea><title>${script}</title>
    <template><template>${script}</template></template><noscript>${script}</noscript>
    <script type=MODULE src="wrong.js"></script><script type=module vite-ignore src="wrong.js"></script>
    <script type=text/plain type=module src="wrong.js"></script>
    <script type=module src="last.js">`
  // When
  const result = await htmlReservationEntries(html, {
    command: 'build',
    root: fs.root,
    filename: join(fs.root, 'index.html'),
    publicDir: false,
  })
  // Then
  expect(result.map(({ request }) => request)).toEqual([
    join(fs.root, 'real.js'),
    join(fs.root, 'real.js'),
    join(fs.root, 'real.js'),
    join(fs.root, 'last.js'),
  ])
})

it('closes inline physical imports after static substitutions without a parent-directory sweep', async () => {
  // Given
  using fs = fixture()
  fs.write('outside/©A.js', `${STYLE}; import './supplier.js'`)
  fs.write('outside/supplier.js', 'export const n=4')
  fs.write('unused.js', STYLE)
  // When
  const entries = await htmlReservationEntries(
    `<script type=module src="%BASE_URL%outside/%NAME%?v=1#x"></script>
    <script type=module>import "../outside/©A.js"; export * from '../outside/supplier.js'; import('https://example.org/a.js')</script>`,
    {
      command: 'build',
      root: fs.root,
      filename: join(fs.root, 'pages/index.html'),
      publicDir: false,
      env: { BASE_URL: '/', NAME: '©A.js' },
    },
  )
  const inventory = collectReservationInputs({
    roots: [],
    entries,
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  })
  // Then
  expect(inventory.inputs.map(({ physicalPath }) => physicalPath)).toEqual([
    join(fs.root, 'outside/supplier.js'),
    join(fs.root, 'outside/©A.js'),
  ])
})

it('excludes public external ignored and empty requests while preserving native inline fallbacks', async () => {
  // Given
  using fs = fixture()
  fs.write('public/public.js', STYLE)
  const html = `<script type=module src=/public.js></script>
    <script type=module src=https://example.org/a.js></script>
    <script type=module src=//example.org/a.js></script>
    <script type=module src=data:text/javascript,1></script>
    <script type=module src=#anchor></script>
    <script type=module src=?query></script>
    <script type=module vite-ignore>import './wrong.js'</script>
    <script type=module src=/public.js>import './inline.js'</script>
    <script type=module src=https://example.org/a.js>import './external-inline.js'</script>
    <script type=module src="">import './empty-inline.js'; import '#internal'; import '?query'</script>`
  // When
  const entries = await htmlReservationEntries(html, {
    command: 'build',
    root: fs.root,
    filename: join(fs.root, 'index.html'),
    publicDir: join(fs.root, 'public'),
  })
  // Then
  expect(entries.map(({ request }) => request)).toEqual([
    './inline.js',
    './external-inline.js',
    './empty-inline.js',
  ])
})

it('leaves unknown substitutions untouched and stringifies known scalar env values', async () => {
  // Given
  using fs = fixture()
  // When
  const entries = await htmlReservationEntries(
    '<script type=module src="%UNKNOWN%/%N%/%B%.js"></script>',
    {
      command: 'build',
      root: fs.root,
      filename: join(fs.root, 'index.html'),
      publicDir: false,
      env: { N: 2, B: false },
    },
  )
  // Then
  expect(entries.map(({ request }) => request)).toEqual([
    join(fs.root, '%UNKNOWN%/2/false.js'),
  ])
})
