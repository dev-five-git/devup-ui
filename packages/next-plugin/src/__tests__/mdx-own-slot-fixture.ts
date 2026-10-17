import { writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { fixture } from './mdx-boundary-fixture.test'

export function fullFixture(body: string, pitch = '') {
  const request = fixture('@mdx-js/loader', { jsx: true })
  const compiler = request.pipeline.loaders[0]
  if (!compiler) throw new TypeError('missing installed compiler')
  const raw = join(request.root, 'raw.cjs')
  const normal =
    body === 'return source'
      ? `require(${JSON.stringify(resolve(import.meta.dir, 'mdx-own-slot-normals.cjs'))}).default`
      : `function(source, map) { ${body} }`
  writeFileSync(
    raw,
    `module.exports = { default: ${normal}, raw: true, pitch: function() { ${pitch} } }`,
  )
  const own = { loader: resolve(import.meta.dir, '../../dist/loader.cjs') }
  return {
    ...request,
    invocation: {
      resource: request.filename,
      ownIndex: 0,
      compilerIndex: 1,
      loaders: [own, compiler, { loader: raw }] as const,
    },
  }
}
