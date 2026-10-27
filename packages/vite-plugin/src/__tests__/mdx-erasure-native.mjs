import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { build } from 'vite'

import { DevupUI } from '../../dist/index.mjs'
import { laterAuthorityControls } from './mdx-authority-controls.mjs'

const landing = createRequire(
  resolve(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const { default: mdx } = await import(
  pathToFileURL(landing.resolve('@mdx-js/rollup')).href
)
const single = process.argv[2] === 'true'
const root = realpathSync
  .native(mkdtempSync(resolve(tmpdir(), 'devup-mdx-erasure-')))
  .replaceAll('\\', '/')
const page = `${root}/src/page.tsx`
const description = `${root}/src/Description.mdx`
const request = './Description.mdx'
const source = `import {Text} from '@devup-ui/react'; import Description from '${request}'; console.info(<Text bg="red"><Description/></Text>);`
const importOnly = `import '${request}';`
const roots = {
  erased: importOnly,
  default: `import Description from '${request}'; console.info(Description);`,
  unused: `import Description from '${request}';`,
  named: `import {value} from '${request}';`,
  namespace: `import * as Content from '${request}';`,
  forwarding: `export {default} from '${request}';`,
  require: `${importOnly} require('./side.js');`,
  dynamic: `${importOnly} import('./side.js');`,
  computed: `${importOnly} import(globalThis.path);`,
  function: `${importOnly} function retained() {}`,
  attributed: `import '${request}' with {type:'json'};`,
  changed: "import './Other.mdx';",
  empty: '',
  multiple: `${importOnly} import './second.tsx';`,
  alias: importOnly,
  unrelated: `${importOnly} import './Other.mdx';`,
  barrel: "import './forward.ts';",
  'later-bypass': `${importOnly} import ${JSON.stringify(description)}; import './widget.js';`,
  'later-nested': `${importOnly} import ${JSON.stringify(description)}; import './widget.js';`,
  epoch: importOnly,
}
mkdirSync(`${root}/src`)
writeFileSync(`${root}/devup.json`, '{}')
writeFileSync(description, '# Erased content')
writeFileSync(`${root}/src/Other.mdx`, '# Other content')
writeFileSync(`${root}/src/side.js`, 'console.info("side effect");')
writeFileSync(`${root}/src/widget.js`, 'export default 1;')
writeFileSync(
  `${root}/src/forward.ts`,
  "export {default} from './Description.mdx';",
)
writeFileSync(`${root}/src/second.tsx`, source)
writeFileSync(page, source)

async function attempt(
  kind,
  devup = DevupUI({ singleCss: single }),
  epoch = 1,
) {
  writeFileSync(
    page,
    kind === 'barrel' ? source.replace(request, './forward.ts') : source,
  )
  const parsed = []
  let failure
  let output
  try {
    output = await build({
      root,
      configFile: false,
      logLevel: 'silent',
      resolve: {
        alias: [
          ...(['alias', 'unrelated'].includes(kind)
            ? [{ find: request, replacement: `${root}/src/widget.js` }]
            : []),
          ...[
            'react/jsx-runtime',
            'react/jsx-dev-runtime',
            'react',
            '@devup-ui/react',
          ].map((name) => ({ find: name, replacement: landing.resolve(name) })),
        ],
      },
      plugins: [
        ...laterAuthorityControls(
          kind,
          { page, widget: `${root}/src/widget.js`, description },
          [],
        ),
        devup,
        mdx(),
        {
          name: 'native-final-demand-erasure',
          enforce: 'post',
          transform(_code, id) {
            if (id === page) return { code: roots[kind], map: null }
            if (id === description)
              return { code: "import './side.js';", map: null }
          },
          moduleParsed(info) {
            if (info.id === page || info.id === description)
              parsed.push({
                id: info.id,
                code: info.code,
                importedIds: info.importedIds,
              })
          },
          options(options) {
            if (epoch !== 2) return
            for (const plugin of options.plugins) {
              if (plugin?.name !== 'devup-ui:mdx') continue
              const hook = plugin.transform
              assert.equal(typeof hook, 'object')
              const handler = hook.handler
              plugin.transform = {
                ...hook,
                handler(code, id, ...args) {
                  if (id !== description)
                    return handler.call(this, code, id, ...args)
                },
              }
            }
          },
        },
      ],
      build: { write: false, minify: false, rolldownOptions: { input: page } },
    })
  } catch (error) {
    failure = error
  }
  const details =
    failure instanceof Error
      ? `${failure.message}\n${JSON.stringify(failure.errors)}`
      : ''
  const positive = kind === 'erased' || (kind === 'epoch' && epoch === 1)
  console.info(
    JSON.stringify({
      event: 'native-erased-demand',
      kind,
      single,
      epoch,
      positive,
      details,
      parsed,
    }),
  )
  if (positive) {
    assert.equal(failure, undefined, details)
    assert.equal(parsed.find((row) => row.id === page)?.code, importOnly)
    assert.equal(
      parsed.find((row) => row.id === description)?.code,
      "import './side.js';",
    )
    assert.ok(output && !Array.isArray(output) && 'output' in output)
    assert.ok(
      output.output.every(
        (row) =>
          row.type !== 'chunk' ||
          !row.code.includes('Cannot run on the runtime'),
      ),
    )
  } else {
    assert.ok(
      failure instanceof Error,
      `${kind}: unproven surviving demand must fail`,
    )
    assert.match(details, /(?:page\.tsx|second\.tsx):\d+:\d+:/)
    assert.match(details, /deferred default cannot use/)
  }
}

try {
  for (const kind of process.env.DEVUP_ERASURE_CASE
    ? [process.env.DEVUP_ERASURE_CASE]
    : Object.keys(roots)) {
    // Given original authored demand, When native transforms erase selected code,
    // Then only the complete direct-root erasure may succeed.
    if (kind === 'epoch') {
      const devup = DevupUI({ singleCss: single })
      await attempt(kind, devup)
      await attempt(kind, devup, 2)
    } else await attempt(kind)
  }
} finally {
  rmSync(root, { recursive: true, force: true })
}

if (!process.env.DEVUP_ERASURE_CASE) await import('./mdx-erasure-history.mjs')
