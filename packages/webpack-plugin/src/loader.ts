import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  createModuleResolver,
  getFileNumByFilename,
  isMdxSource,
  type ModuleAliasOptions,
  remapMdxError,
  type ResolutionInputObserver,
  resolutionWatchPath,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  setModuleResolver,
} from '@devup-ui/wasm'
import type { LoaderContext, RawLoaderDefinitionFunction } from 'webpack'

import { withCompilerScope } from './build-scope'
import { createGeneratedOutputWriter, normalizeSheet } from './generated-output'

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
  alias?: ModuleAliasOptions
  symlinks?: boolean
  configurationFiles?: readonly string[]
}

function toLoaderError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

type LoaderSourceMap = Exclude<
  Parameters<LoaderContext<DevupUILoaderOptions>['callback']>[2],
  string | undefined
>

function parseSourceMap(sourceMap: string | undefined): LoaderSourceMap {
  if (!sourceMap) return null

  return JSON.parse(sourceMap)
}

const stateWriter = createGeneratedOutputWriter()
/** Resolve imports to the cwd-relative ids this loader extracts files under */
function setCwdModuleResolver(options: {
  readonly rootDir: string
  readonly conditions: readonly string[]
  readonly mdxExtensions: readonly string[]
  readonly alias: ModuleAliasOptions | undefined
  readonly onResolutionInputs: ResolutionInputObserver
}): void {
  const { rootDir, conditions, mdxExtensions, alias, onResolutionInputs } =
    options
  const moduleResolver = createModuleResolver({
    cwd: rootDir,
    includeMdx: mdxExtensions,
    conditions,
    alias,
    onResolutionInputs,
    toId: (path) => relative(rootDir, path).replaceAll('\\', '/'),
  })
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
      alias,
      symlinks = true,
      configurationFiles = [],
    } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath

    if (watch) {
      this.addDependency(sheetFile)
      this.addDependency(classMapFile)
      this.addDependency(fileMapFile)
      for (const file of configurationFiles) this.addDependency(file)
    }

    try {
      withCompilerScope(this._compiler, () => {
        let relCssDir = relative(dirname(id), cssDir).replaceAll('\\', '/')

        // POSIX-normalize so the engine's bucket key matches the canonical map /
        // FILE_ROUTES keys (built with forward slashes by plugin-utils). Without
        // this, single-importer collapse and atom hoisting silently no-op on
        // Windows. No-op on POSIX.
        const relativePath = relative(rootDir, id).replaceAll('\\', '/')

        if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
        setCwdModuleResolver({
          rootDir,
          conditions,
          mdxExtensions,
          alias,
          onResolutionInputs: (inputs) => {
            for (const file of inputs.fileDependencies)
              this.addDependency(resolutionWatchPath(file, !symlinks))
            for (const path of inputs.missingDependencies)
              this.addMissingDependency(resolutionWatchPath(path, !symlinks))
          },
        })
        const output = codeExtract(
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
        const {
          code,
          css = '',
          map,
          cssFile,
          updatedBaseStyle,
          dependencies = [],
        } = (() => {
          try {
            return {
              code: output.code,
              css: output.css,
              map: output.map,
              cssFile: output.cssFile,
              updatedBaseStyle: output.updatedBaseStyle,
              dependencies: output.dependencies,
            }
          } finally {
            output.free()
          }
        })()
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
          const targetPath = join(cssDir, basename(cssFile))
          promises.push(
            stateWriter.write(
              targetPath,
              watch ? getCss(getFileNumByFilename(targetPath), true) : css,
            ),
          )
          if (watch) {
            promises.push(
              stateWriter.write(sheetFile, normalizeSheet(exportSheet())),
              stateWriter.write(classMapFile, exportClassMap()),
              stateWriter.write(fileMapFile, exportFileMap()),
            )
          }
        }
        Promise.all(promises).then(
          () => callback(null, code, sourceMap),
          (error) => callback(toLoaderError(error)),
        )
      })
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
