import {
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  realpath,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'
import { build } from 'vite'

import { type NativePresence, NonphysicalModules } from '../nonphysical-modules'
import { DevupUI } from '../plugin'

type NativeNode = ReturnType<NativePresence['getModuleInfo']>
function nativePresence(
  nodes: ReadonlyMap<string, NativeNode>,
): NativePresence {
  return {
    getModuleIds: () => nodes.keys(),
    getModuleInfo: (id) => nodes.get(id) ?? null,
  }
}

it.each([
  'cached',
  'replaced',
  'invalidated',
  'orphaned',
  'unavailable',
] as const)(
  'retains absent-O ready attachments only with current consumer evidence when %s (UNIT)',
  async (state) => {
    // Given a published real observation and an in-memory native presence implementation.
    const modules = new NonphysicalModules([])
    const root = '\0unit:root.js'
    const child = '\0unit:child.js'
    const consumer = {
      code: 'export {}',
      isEntry: true,
      importedIds: [],
      dynamicallyImportedIds: [],
    }
    const nodes = new Map<string, NativeNode>([[root, consumer]])
    const graph = nativePresence(nodes)
    modules.direct(root, root, { code: 'export {}' })
    modules.resolved(root, { path: child, code: 'export const value=1' })
    modules.publish(await modules.finish(graph, []))
    modules.start()
    switch (state) {
      case 'cached':
        break
      case 'replaced':
        modules.direct(root, root, { code: 'export {}' })
        break
      case 'invalidated':
        modules.invalidate(child)
        break
      case 'orphaned':
        nodes.clear()
        break
      case 'unavailable':
        nodes.set(root, { ...consumer, code: null })
        break
      default:
        state satisfies never
    }
    // When a completed interval has no current observation of the ready child.
    const completed = await modules.finish(graph, [child])
    // Then only the unchanged, reachable, code-bearing consumer retains that exact ID.
    expect(modules.observations()).not.toContain(child)
    expect(completed.ids.includes(child)).toBe(state === 'cached')
  },
)

it('retains existing unreachable physical files and reachable opaque code in an empty-O interval (UNIT)', async () => {
  // Given actual files plus reachable, cyclic and orphan native vertices.
  const root = (
    await realpath(await mkdtemp(join(tmpdir(), 'devup-v1-retention-')))
  ).replaceAll('\\', '/')
  const physical = `${root}/existing.js`
  await writeFile(physical, 'export {}')
  const opaque = '\0unit:present.js'
  const absent = `${root}/missing.js`
  const directory = root
  const nodes = new Map<string, NativeNode>([
    [
      opaque,
      {
        code: '',
        isEntry: true,
        importedIds: [opaque, 'missing'],
        dynamicallyImportedIds: [],
      },
    ],
    [
      'orphan',
      { code: '', isEntry: false, importedIds: [], dynamicallyImportedIds: [] },
    ],
  ])
  const modules = new NonphysicalModules([])
  try {
    // When the real retention implementation probes physical history and walks native reachability.
    const completed = await modules.finish(nativePresence(nodes), [
      physical,
      absent,
      directory,
      `${physical}/child.js`,
      opaque,
      'orphan',
    ])
    // Then only the existing file and reachable code survive, without invented observations.
    expect(modules.observations()).toEqual([])
    expect(completed.ids.toSorted()).toEqual([physical, opaque].sort())
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

it('keeps existing direct and recursive extraction filenames consistent across queried native builds', async () => {
  // Given a real typed source, plain typed constant and native query variants.
  const root = (
    await realpath(await mkdtemp(join(tmpdir(), 'devup-v1-current-id-')))
  ).replaceAll('\\', '/')
  const entry = join(root, 'src/main.ts')
  const style = `${root}/src/style.ts`
  const constant = `${root}/src/constant.ts`
  const direct: {
    readonly id: string
    readonly code: string
    readonly css: string | undefined
    readonly map: string | undefined
  }[][] = []
  const recursive: string[][] = []
  const nativeCss: {
    readonly fileName: string
    readonly source: string | Uint8Array
  }[][] = []
  const extract = wasm.codeExtract
  const installResolver = wasm.setModuleResolver
  let index = 0
  const extraction = spyOn(wasm, 'codeExtract').mockImplementation(
    (...args) => {
      const output = extract(...args)
      direct[index]?.push({
        id: args[0],
        code: output.code,
        css: output.css,
        map: output.map,
      })
      return output
    },
  )
  const resolution = spyOn(wasm, 'setModuleResolver').mockImplementation(
    (resolver) => {
      if (typeof resolver !== 'function') {
        installResolver(resolver)
        return
      }
      installResolver((specifier: string, importer: string) => {
        const result: unknown = resolver(specifier, importer)
        if (
          result !== null &&
          typeof result === 'object' &&
          'path' in result &&
          typeof result.path === 'string'
        ) {
          recursive[index]?.push(result.path)
        }
        return result
      })
    },
  )
  try {
    await mkdir(join(root, 'src'))
    await writeFile(
      join(root, 'package.json'),
      '{"name":"devup-v1-current-id","type":"module"}',
    )
    await writeFile(join(root, 'tsconfig.json'), '{}')
    await writeFile(constant, "export const color: string = 'red';")
    await writeFile(
      style,
      "import {css} from '@devup-ui/react'; import {color} from './constant'; export const cls=css({bg:color});",
    )
    // When actual native builds resolve two queries through today's unmodified plugin.
    for (const query of ['learning', 'first', 'second']) {
      direct.push([])
      recursive.push([])
      await writeFile(
        entry,
        `import {cls} from './style.ts?variant=${query}'; export {cls};`,
      )
      const result = await build({
        root,
        configFile: false,
        logLevel: 'silent',
        plugins: [DevupUI()],
        build: { write: false, lib: { entry, formats: ['es'] } },
      })
      nativeCss.push(
        (Array.isArray(result) ? result : [result]).flatMap((bundle) => {
          if (!('output' in bundle))
            throw new TypeError('Expected native build output, not a watcher')
          return bundle.output.flatMap((output) =>
            output.type === 'asset' && output.fileName.endsWith('.css')
              ? [{ fileName: output.fileName, source: output.source }]
              : [],
          )
        }),
      )
      if (index === 0) {
        const directory = join(root, 'df/numbering')
        const lists: unknown[] = await Promise.all(
          (await readdir(directory)).map(async (file) =>
            JSON.parse(await readFile(join(directory, file), 'utf8')),
          ),
        )
        expect(
          lists.some(
            (list) =>
              Array.isArray(list) &&
              list.some(
                (id: unknown) =>
                  typeof id === 'string' &&
                  direct[0]?.some((call) => call.id === id),
              ),
          ),
        ).toBe(true)
      }
      index += 1
    }
    // Then real direct/resolver calls use physical filenames and produce identical extracted styling.
    const first = direct[1]?.find((call) => call.id === style)
    const second = direct[2]?.find((call) => call.id === style)
    expect(first).toBeDefined()
    expect(second?.id).toBe(first?.id)
    expect(second?.code).toBe(first?.code)
    expect(second?.map).toBe(first?.map)
    expect(nativeCss[1]?.length).toBeGreaterThan(0)
    expect(nativeCss[2]).toEqual(nativeCss[1])
    expect(first?.css).toContain('background:red')
    expect(first?.map).toContain(JSON.stringify(style))
    expect(recursive[1]).toContain(constant)
    expect(recursive[2]).toContain(constant)
    expect(direct[0]?.find((call) => call.id === style)?.code).not.toBe(
      first?.code,
    )
    expect(direct.flat().some((call) => call.id.includes('?'))).toBe(false)
  } finally {
    extraction.mockRestore()
    resolution.mockRestore()
    wasm.setModuleResolver(undefined)
    wasm.resetBuildState()
    await rm(root, { recursive: true, force: true })
  }
}, 30000)
