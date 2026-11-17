import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  computeFileReach,
  conventionalSourceRoots,
  planAtomHoist,
} from '../import-graph'

let root: string

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-ui-source-roots-'))
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

function write(files: Record<string, string>) {
  for (const [path, code] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), code)
  }
}

describe('conventionalSourceRoots', () => {
  it('lists src and app of every project root once', () => {
    expect(conventionalSourceRoots(root, root, join(root, 'web'))).toEqual(
      ['src', 'app', 'web/src', 'web/app'].map((dir) => join(root, dir)),
    )
  })
})

describe('buildStaticImportGraph roots', () => {
  it('links the files of several roots under one source directory', () => {
    write({
      'app/one/page.tsx': "import '../../src/shared'\n",
      'app/two/page.tsx': "import '../../src/shared'\n",
      'src/shared.tsx': 'export const shared = 1\n',
      'scripts/tool.ts': "import '../src/shared'\n",
    })
    const graph = buildStaticImportGraph(
      root,
      undefined,
      conventionalSourceRoots(root, join(root, 'src')),
    )
    expect(graph.files).toEqual(
      ['app/one/page.tsx', 'app/two/page.tsx', 'src/shared.tsx'].map((file) =>
        join(root, file),
      ),
    )
    expect([
      ...(graph.staticImporters.get(join(root, 'src/shared.tsx')) ?? []),
    ]).toEqual(
      ['app/one/page.tsx', 'app/two/page.tsx'].map((file) => join(root, file)),
    )

    // Both routes of `app` reach the shared module of `src`, so it hoists
    const options = { srcDir: root, cwd: root, graph }
    const plan = planAtomHoist(
      buildCanonicalMap(options),
      computeFileReach(options),
      2,
    )
    expect(plan?.reachByBucket['src/shared.tsx']).toEqual([0, 1])
  })
})
