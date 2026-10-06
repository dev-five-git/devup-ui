import { resolve } from 'node:path'

import { mock } from 'bun:test'

import loader, { type DevupUILoaderOptions } from '../loader'

const defaults = {
  package: 'package',
  cssDir: 'cssDir',
  sheetFile: 'sheetFile',
  classMapFile: 'classMapFile',
  fileMapFile: 'fileMapFile',
  themeFile: 'themeFile',
  watch: false,
  singleCss: true,
  defaultSheet: {},
  defaultClassMap: {},
  defaultFileMap: {},
}

export function invoke(
  options: Partial<DevupUILoaderOptions> = {},
  resourcePath = resolve('App.tsx'),
  source = 'source',
) {
  const addDependency = mock()
  const addMissingDependency = mock()
  const callback = mock()
  const result = new Promise<{
    readonly code?: string | undefined
    readonly map?: string | null | undefined
  }>((resolve, reject) => {
    callback.mockImplementation(
      (error: Error | null, code?: string, map?: string | null) => {
        if (error) reject(error)
        else resolve({ code, map })
      },
    )
    Reflect.apply(
      loader,
      {
        getOptions: () => ({ ...defaults, ...options }),
        resourcePath,
        addDependency,
        addMissingDependency,
        async: () => callback,
      },
      [Buffer.from(source)],
    )
  })
  return { result, callback, addDependency, addMissingDependency }
}
