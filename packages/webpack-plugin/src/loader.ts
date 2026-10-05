import { writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  createModuleResolver,
  createStateWriter,
  isMdxSource,
  remapMdxError,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  setModuleResolver,
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
  rootDir?: string
  conditions?: readonly string[]
  mdxExtensions?: readonly string[]
}

function toLoaderError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

function parseSourceMap(sourceMap: string | undefined): string | null {
  if (!sourceMap) return null

  JSON.parse(sourceMap)
  return sourceMap
}

const stateWriter = createStateWriter((path, content, encoding) =>
  encoding ? writeFile(path, content, encoding) : writeFile(path, content),
)
const moduleResolvers = new Map<
  string,
  ReturnType<typeof createModuleResolver>
>()

/** Resolve imports to the cwd-relative ids this loader extracts files under */
function setCwdModuleResolver(
  rootDir: string,
  conditions: readonly string[],
  mdxExtensions: readonly string[],
): void {
  const key = JSON.stringify([rootDir, conditions, mdxExtensions])
  let moduleResolver = moduleResolvers.get(key)
  if (!moduleResolver) {
    moduleResolver = createModuleResolver({
      cwd: rootDir,
      includeMdx: mdxExtensions,
      conditions,
      toId: (path) => relative(rootDir, path).replaceAll('\\', '/'),
    })
    moduleResolvers.set(key, moduleResolver)
  }
  setModuleResolver(moduleResolver)
}

const devupUILoader: RawLoaderDefinitionFunction<DevupUILoaderOptions> =
  function (source, inputSourceMap) {
    const {
      watch,
      package: libPackage,
      cssDir,
      sheetFile,
      classMapFile,
      fileMapFile,
      singleCss,
      importAliases = {},
      rootDir = process.cwd(),
      conditions = ['import', 'module', 'node'],
      mdxExtensions = ['.mdx'],
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

      // POSIX-normalize so the engine's bucket key matches the canonical map /
      // FILE_ROUTES keys (built with forward slashes by plugin-utils). Without
      // this, single-importer collapse and atom hoisting silently no-op on
      // Windows. No-op on POSIX.
      const relativePath = relative(rootDir, id).replaceAll('\\', '/')

      if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
      setCwdModuleResolver(rootDir, conditions, mdxExtensions)
      const {
        code,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
        dependencies = [],
      } = codeExtract(
        relativePath,
        source.toString(),
        libPackage,
        relCssDir,
        singleCss,
        false,
        true,
        importAliases,
        ...(isMdxSource(id, mdxExtensions)
          ? (['compiled-mdx'] as const)
          : ([] as const)),
      )
      for (const dependency of dependencies) {
        this.addDependency(resolve(rootDir, dependency))
      }
      const sourceMap = parseSourceMap(map)
      const promises: Promise<void>[] = []
      if (updatedBaseStyle) {
        // update base style
        promises.push(
          stateWriter.write(
            join(cssDir, 'devup-ui.css'),
            getCss(null, false),
            'utf-8',
          ),
        )
      }
      if (cssFile) {
        const content = `${this.resourcePath} ${Date.now()}`
        // should be reset css
        promises.push(
          stateWriter.write(
            join(cssDir, basename(cssFile)),
            watch ? `/* ${content} */` : css,
          ),
        )
        if (watch) {
          promises.push(
            stateWriter.write(sheetFile, exportSheet()),
            stateWriter.write(classMapFile, exportClassMap()),
            stateWriter.write(fileMapFile, exportFileMap()),
          )
        }
      }
      Promise.all(promises).then(
        () => callback(null, code, sourceMap as Parameters<typeof callback>[2]),
        (error) => callback(toLoaderError(error)),
      )
    } catch (error) {
      callback(
        isMdxSource(id, mdxExtensions)
          ? remapMdxError(
              error,
              relative(rootDir, id).replaceAll('\\', '/'),
              inputSourceMap,
            )
          : toLoaderError(error),
      )
    }
    return
  }
export default devupUILoader
