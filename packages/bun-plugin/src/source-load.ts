import { dirname, relative } from 'node:path'

import {
  type createModuleResolver,
  isMdxSource,
  isProjectJsx,
  type PreparedSource,
  remapMdxError,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  hasDevupUI,
  setDebug,
  setModuleResolver,
} from '@devup-ui/wasm'

import type { createMdxPreparation } from './mdx'
import type { createMdxOwnership } from './mdx-ownership'
import {
  importAliases,
  importsCompiledPackage,
  libPackage,
  preserveDependencies,
  sourceLoader,
} from './source'

export interface SourceProject {
  readonly root: string
  readonly cssDir: string
  readonly debug: boolean
  readonly resolver: ReturnType<typeof createModuleResolver>
  readonly mdxExtensions: readonly string[]
  readonly prepared: Map<string, Exclude<PreparedSource, undefined>>
  readonly ownership: ReturnType<typeof createMdxOwnership>
  readonly mdx: ReturnType<typeof createMdxPreparation>
  readonly reserve?: (preparedChanged: boolean) => void
}

export async function loadSourceFile(filePath: string, project: SourceProject) {
  const original = await Bun.file(filePath).text()
  const selectedMdx = isMdxSource(filePath, project.mdxExtensions)
  const mdx = selectedMdx
    ? await project.mdx.prepare(filePath, original)
    : undefined
  const loader = mdx ? 'jsx' : sourceLoader(filePath)
  const contents = mdx?.value ?? original
  project.ownership.observe(
    filePath,
    contents,
    !mdx && ['ts', 'tsx'].includes(loader),
  )
  const admitted = await project.mdx.run(() => {
    setModuleResolver(project.resolver)
    return (
      selectedMdx ||
      isProjectJsx(filePath, importAliases) ||
      importsCompiledPackage(contents, loader) ||
      hasDevupUI(filePath, contents, libPackage, importAliases)
    )
  })
  if (admitted === undefined) return { contents: original, loader }
  if (admitted) {
    const extract = () => {
      setDebug(project.debug)
      setModuleResolver(project.resolver)
      const output = codeExtract(
        filePath,
        contents,
        libPackage,
        relative(dirname(filePath), project.cssDir).replaceAll('\\', '/'),
        true,
        true,
        false,
        importAliases,
        ...(mdx ? (['compiled-mdx'] as const) : ([] as const)),
      )
      const { code, dependencies } = (() => {
        try {
          return { code: output.code, dependencies: output.dependencies }
        } finally {
          output.free()
        }
      })()
      return {
        contents: preserveDependencies(code, filePath, dependencies),
        loader,
      }
    }
    try {
      return (
        (await project.mdx.run(extract, () =>
          project.reserve?.(selectedMdx),
        )) ?? { contents: original, loader }
      )
    } catch (cause) {
      const error = project.resolver.remapError(cause)
      if (mdx) throw remapMdxError(error, filePath, mdx.map)
      throw error
    }
  }
  return { contents, loader }
}
