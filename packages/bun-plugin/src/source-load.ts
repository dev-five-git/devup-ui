import { dirname, relative } from 'node:path'

import {
  type createModuleResolver,
  isMdxSource,
  type PreparedSource,
  remapMdxError,
} from '@devup-ui/plugin-utils'
import { codeExtract, setDebug, setModuleResolver } from '@devup-ui/wasm'

import { compileMdx, MdxCompilerError } from './mdx'
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
  readonly prepared: Map<string, PreparedSource>
  readonly ownership: ReturnType<typeof createMdxOwnership>
}

export async function loadSourceFile(filePath: string, project: SourceProject) {
  const original = await Bun.file(filePath).text()
  const selectedMdx = isMdxSource(filePath, project.mdxExtensions)
  const mdx = selectedMdx
    ? await compileMdx(project.root, filePath, original)
    : undefined
  if (selectedMdx && !mdx) throw new MdxCompilerError(filePath, undefined)
  const loader = mdx ? 'jsx' : sourceLoader(filePath)
  const contents = mdx?.value ?? original
  if (mdx)
    project.prepared.set(filePath, {
      code: contents,
      map: mdx.map,
      sourceType: 'compiled-mdx',
    })
  project.ownership.observe(
    filePath,
    contents,
    !mdx && ['ts', 'tsx'].includes(loader),
  )
  if (importsCompiledPackage(contents, loader)) {
    setDebug(project.debug)
    setModuleResolver(project.resolver)
    try {
      const code = codeExtract(
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
      return {
        contents: preserveDependencies(code.code, filePath, code.dependencies),
        loader,
      }
    } catch (cause) {
      const error = project.resolver.remapError(cause)
      if (mdx) throw remapMdxError(error, filePath, mdx.map)
      throw error
    }
  }
  return { contents, loader }
}
