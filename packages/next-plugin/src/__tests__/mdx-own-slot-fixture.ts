import { writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { fixture } from './mdx-boundary-fixture.test'

export function fullFixture(body: string, pitch = '') {
  const request = fixture('@mdx-js/loader', { jsx: true })
  const compiler = request.pipeline.loaders[0]
  if (!compiler) throw new TypeError('missing installed compiler')
  const raw = join(request.root, 'raw.cjs')
  writeFileSync(
    raw,
    `module.exports = function(source, map) { ${body} }; module.exports.raw = true; module.exports.pitch = function() { ${pitch} }`,
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
