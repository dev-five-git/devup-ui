import { writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative } from 'node:path'

import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
} from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

export interface DevupUILoaderOptions {
  package: string
  cssDir: string
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  watch: boolean
  singleCss: boolean
  importAliases?: Record<string, string | null>
}

function toLoaderError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

function parseSourceMap(sourceMap: string | undefined): string | null {
  if (!sourceMap) return null

  JSON.parse(sourceMap)
  return sourceMap
}

const devupUILoader: RawLoaderDefinitionFunction<DevupUILoaderOptions> =
  function (source) {
    const {
      watch,
      package: libPackage,
      cssDir,
      sheetFile,
      classMapFile,
      fileMapFile,
      singleCss,
      importAliases = {},
    } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath

    if (watch) {
      this.addDependency(sheetFile)
      this.addDependency(classMapFile)
      this.addDependency(fileMapFile)
    }

    try {
      let relCssDir = relative(dirname(id), cssDir).replaceAll('\\', '/')

      const relativePath = relative(process.cwd(), id)

      if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
      const {
        code,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
      } = codeExtract(
        relativePath,
        source.toString(),
        libPackage,
        relCssDir,
        singleCss,
        false,
        true,
        importAliases,
      )
      const sourceMap = parseSourceMap(map)
      const promises: Promise<void>[] = []
      if (updatedBaseStyle) {
        // update base style
        promises.push(
          writeFile(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8'),
        )
      }
      if (cssFile) {
        const content = `${this.resourcePath} ${Date.now()}`
        // should be reset css
        promises.push(
          writeFile(
            join(cssDir, basename(cssFile)),
            watch ? `/* ${content} */` : css,
          ),
        )
        if (watch) {
          promises.push(
            writeFile(sheetFile, exportSheet()),
            writeFile(classMapFile, exportClassMap()),
            writeFile(fileMapFile, exportFileMap()),
          )
        }
      }
      Promise.all(promises).then(
        () => callback(null, code, sourceMap as Parameters<typeof callback>[2]),
        (error) => callback(toLoaderError(error)),
      )
    } catch (error) {
      callback(toLoaderError(error))
    }
    return
  }
export default devupUILoader
