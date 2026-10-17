import { readFile } from 'node:fs'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { isMdxRecord, type MdxLoader } from '../mdx-pipeline'
import type { RunLoaders } from '../mdx-prepare-runner'

type NativeControlRunner = (
  options: Omit<Parameters<RunLoaders>[0], 'readResource'> & {
    readonly readResource: (
      path: string,
      callback: (error: NodeJS.ErrnoException | null, source?: Buffer) => void,
    ) => void
  },
  callback: Parameters<RunLoaders>[1],
) => void

function isNativeControlRunner(value: unknown): value is NativeControlRunner {
  return typeof value === 'function'
}

export function completeNativeAsync(filename: string, loader: MdxLoader) {
  const installed = createRequire(
    resolve(import.meta.dir, '../../../../apps/landing/package.json'),
  )
  const runner: unknown = installed(
    'next/dist/compiled/loader-runner/LoaderRunner.js',
  )
  if (!isMdxRecord(runner) || !isNativeControlRunner(runner.runLoaders))
    throw new TypeError('missing installed native runner')
  const runLoaders = runner.runLoaders
  const context = {}
  return new Promise<unknown>((done, reject) => {
    runLoaders(
      {
        resource: filename,
        loaders: [loader],
        context,
        readResource(path, callback) {
          readFile(path, (error, source) => {
            callback(error, source)
            // Native normal has now requested async completion; release its real callback.
            const finish: unknown = Reflect.get(context, 'callback')
            if (typeof finish !== 'function')
              throw new TypeError('missing native completion')
            Reflect.apply(finish, context, [null, source])
          })
        },
      },
      (error, result) => (error ? reject(error) : done(result)),
    )
  })
}
