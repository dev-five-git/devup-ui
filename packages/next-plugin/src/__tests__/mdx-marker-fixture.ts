import { readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'

import { compileMdx } from '../mdx-prepare'
import { captureMdxTurboContext } from '../mdx-turbo-context'
import { fixture } from './mdx-boundary-fixture.test'

export function markerFixture(
  entry = '@next/mdx/mdx-js-loader',
  sourceMap = true,
) {
  const request = fixture(entry)
  const loader = request.pipeline.loaders[0]
  if (!loader) throw new TypeError('missing compiler')
  const count = join(request.root, 'constructions.txt')
  writeFileSync(count, '')
  const plugin = join(request.root, 'remark.cjs')
  writeFileSync(
    plugin,
    `module.exports = function(options) { require('node:fs').appendFileSync(options.count, 'x'); return function(tree) { tree.children[0].children[0].value = options.text } }`,
  )
  const receiving = {
    rootContext: request.root,
    sourceMap,
    _module: { __reserved: 'TurbopackContext' },
  }
  const context = {
    owner: {},
    generation: {},
    ...captureMdxTurboContext(loader.loader, receiving),
  }
  const options = (text: string, named = plugin) => ({
    format: 'mdx',
    remarkPlugins: [[named, { text, count }]],
  })
  const installed = createRequire(join(request.root, 'package.json'))
  const original: unknown = installed(loader.loader)
  if (typeof original !== 'function')
    throw new TypeError('missing native MDX entry')
  return {
    ...request,
    context,
    loader,
    plugin,
    receiving,
    options,
    pipeline: { ...request.pipeline, bundler: 'turbo' as const },
    count: () => readFileSync(count, 'utf8').length,
    prepare(text: string, named = plugin) {
      return compileMdx({
        ...request,
        context: { ...context, owner: {}, generation: {} },
        pipeline: {
          ...request.pipeline,
          bundler: 'turbo',
          loaders: [{ ...loader, options: options(text, named) }],
        },
      })
    },
    native(text: string, named = plugin) {
      return new Promise<{ readonly source: string; readonly map: unknown }>(
        (done, reject) => {
          const callback = (
            error: unknown,
            source: string | Buffer,
            map: unknown,
          ) => {
            if (error) {
              reject(error)
              return
            }
            done({ source: source.toString(), map })
          }
          original.call(
            {
              ...receiving,
              context: dirname(request.filename),
              resourcePath: request.filename,
              getOptions: () => options(text, named),
              async: () => callback,
            },
            readFileSync(request.filename, 'utf8'),
          )
        },
      )
    },
  }
}
