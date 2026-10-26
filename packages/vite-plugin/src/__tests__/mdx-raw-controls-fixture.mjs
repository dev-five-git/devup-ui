import { mkdirSync, mkdtempSync, realpathSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const landing = createRequire(
  resolve(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const { default: mdx } = await import(
  pathToFileURL(landing.resolve('@mdx-js/rollup')).href
)

export function createRawControlFixture() {
  const root = realpathSync
    .native(mkdtempSync(resolve(tmpdir(), 'devup-mdx-raw-control-')))
    .replaceAll('\\', '/')
  const page = `${root}/src/page.tsx`
  const description = `${root}/src/Description.mdx`
  const widget = `${root}/src/widget.js`
  const unrelated = `${root}/src/Unrelated.mdx`
  mkdirSync(`${root}/src`)
  writeFileSync(`${root}/devup.json`, '{}')
  writeFileSync(description, '# Actual current MDX')
  writeFileSync(widget, 'export default 0;')
  writeFileSync(unrelated, '# Independently compiled unrelated MDX')
  writeFileSync(
    `${root}/src/other.js`,
    `import ${JSON.stringify(description)}; import ${JSON.stringify(unrelated)}; console.info('other');`,
  )
  return { root, page, description, widget, unrelated, landing, mdx }
}
