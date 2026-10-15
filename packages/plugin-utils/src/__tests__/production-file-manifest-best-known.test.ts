import { mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import type { PrepareSource } from '../import-graph'
import { PreparedSourceTypeError } from '../prepared-source'
import { collectProductionFileManifest } from '../production-file-manifest'
import { enumerateProductionSourceFiles } from '../production-source-files'

it('rejects invalid prepared source types despite reserve-only opt-in', async () => {
  // Given a malformed boundary object, not missing compiler output.
  const prepared = Object.defineProperty({ code: 'export {}' }, 'sourceType', {
    value: 'invalid-type',
  })
  const selected = {
    ...context(() => prepared),
    unpreparedMarkdown: 'reserve-only' as const,
  }
  // When the actual prepared-source parser receives that object.
  const operation = collectProductionFileManifest({ contexts: [selected] })
  // Then reserve-only cannot turn invalid preparation into physical-only success.
  await expect(operation).rejects.toMatchObject({
    cause: expect.any(PreparedSourceTypeError),
  })
})

let root: string
let page: string
let leaf: string
beforeEach(async () => {
  root = await realpath(await mkdtemp(join(tmpdir(), 'devup-v1-best-known-')))
  page = join(root, 'page.mdx')
  leaf = join(root, 'leaf.js')
  await writeFile(page, '# Documentation\n\nimport "./missing.js"')
  await writeFile(leaf, 'export const value=1')
})
afterEach(async () => {
  await rm(root, { recursive: true, force: true })
})
function context(prepareSource?: PrepareSource) {
  return {
    key: 'native',
    files: enumerateProductionSourceFiles({
      roots: [],
      sourceFiles: [page],
      includeMdx: true,
    }),
    resolverOptions: { cwd: root, includeMdx: true },
    toId: (path: string) => path.replaceAll('\\', '/'),
    ...(prepareSource === undefined ? {} : { prepareSource }),
  }
}

it('keeps missing Markdown preparation fatal under the strict default', async () => {
  // Given selected actual Markdown without compiler output.
  const selected = context()
  // When collecting without the integration opt-in.
  const operation = collectProductionFileManifest({ contexts: [selected] })
  // Then source-located strict rejection survives.
  await expect(operation).rejects.toThrow(`${page}:1:1`)
})

it('reserves unprepared Markdown without reading it as JavaScript when opted in', async () => {
  // Given a selected source whose raw text contains an unresolved import.
  const observed: string[] = []
  const selected = {
    ...context(),
    ...{ unpreparedMarkdown: 'reserve-only' as const },
    resolverOptions: {
      cwd: root,
      includeMdx: true,
      onResolutionInputs(inputs: {
        readonly fileDependencies: readonly string[]
      }) {
        observed.push(...inputs.fileDependencies)
      },
    },
  }
  // When reservation uses the explicit integration policy.
  const files = await collectProductionFileManifest({ contexts: [selected] })
  // Then exact physical membership and source observation survive without fake compiled output.
  expect(files.map(({ path }) => path)).toEqual([page])
  expect(observed).toContain(page)
  expect(await readFile(page, 'utf8')).toBe(
    '# Documentation\n\nimport "./missing.js"',
  )
})

it.each(['', "export {value} from './leaf.js'"])(
  'scans genuine prepared JavaScript %j rather than treating it as unprepared',
  async (code) => {
    // Given actual supplied compiled output under the Markdown identity.
    const selected = {
      ...context((path) => (path === page ? code : undefined)),
      ...{ unpreparedMarkdown: 'reserve-only' as const },
    }
    // When the prepared branch collects ordinary compiler edges.
    const files = await collectProductionFileManifest({ contexts: [selected] })
    // Then only the real prepared import can add the physical child.
    expect(files.map(({ path }) => path).sort()).toEqual(
      (code ? [leaf, page] : [page]).sort(),
    )
  },
)

it('propagates compiler rejection despite reserve-only opt-in', async () => {
  // Given an actual asynchronous preparation fault.
  const cause = new RangeError('compiler failed')
  const selected = {
    ...context(async () => {
      throw cause
    }),
    ...{ unpreparedMarkdown: 'reserve-only' as const },
  }
  // When preparation fails before any reservation fallback.
  const operation = collectProductionFileManifest({ contexts: [selected] })
  // Then the existing preparation failure retains its cause.
  await expect(operation).rejects.toMatchObject({ cause })
})
