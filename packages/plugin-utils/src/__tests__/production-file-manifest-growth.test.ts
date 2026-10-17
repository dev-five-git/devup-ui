import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { enumerateProductionSourceFiles } from '../production-source-files'

type Shape = 'diamond' | 'ladder'
type Edge = readonly [string, string]
type Graph = {
  readonly vertices: readonly string[]
  readonly edges: readonly Edge[]
  readonly V: number
  readonly E: number
}

function graph(shape: Shape, levels: number): Graph {
  const vertices = ['src/a.ts', 'src/b.ts', 'outside/leaf.js']
  const edges: Edge[] = []
  switch (shape) {
    case 'diamond':
      for (const origin of ['src/a.ts', 'src/b.ts'])
        edges.push([origin, 'outside/0-hub.js'])
      for (let level = 0; level < levels; level++) {
        const hub = `outside/${level}-hub.js`
        vertices.push(hub)
        for (const side of ['left', 'right']) {
          const arm = `outside/${level}-${side}.js`
          vertices.push(arm)
          edges.push([hub, arm])
          edges.push([
            arm,
            level + 1 === levels
              ? 'outside/leaf.js'
              : `outside/${level + 1}-hub.js`,
          ])
        }
      }
      return { vertices, edges, V: 3 * levels + 3, E: 4 * levels + 2 }
    case 'ladder':
      for (const origin of ['src/a.ts', 'src/b.ts'])
        for (const side of ['left', 'right'])
          edges.push([origin, `outside/0-${side}.js`])
      for (let level = 0; level < levels; level++) {
        for (const side of ['left', 'right']) {
          const vertex = `outside/${level}-${side}.js`
          vertices.push(vertex)
          const targets =
            level + 1 === levels
              ? ['outside/leaf.js']
              : [
                  `outside/${level + 1}-left.js`,
                  `outside/${level + 1}-right.js`,
                ]
          for (const target of targets) edges.push([vertex, target])
        }
      }
      return { vertices, edges, V: 2 * levels + 3, E: 4 * levels + 2 }
    default: {
      const unreachable: never = shape
      return unreachable
    }
  }
}

class GrowthBudgetExceeded extends RangeError {
  readonly name = 'GrowthBudgetExceeded'
  constructor(
    readonly meter: 'file' | 'filesystem',
    readonly count: number,
    readonly budget: number,
  ) {
    super(`${meter} growth budget exceeded: ${count} > ${budget}`)
  }
}

const cases = (['diamond', 'ladder'] as const).flatMap((shape) =>
  [3, 6, 9, 12, 18].flatMap((levels) =>
    (['AB', 'BA'] as const).map((order) => ({ shape, levels, order })),
  ),
)

it.each(cases)(
  'bounds actual external $shape at $levels levels when seeds arrive $order',
  async ({ shape, levels, order }) => {
    // Given independently declared vertices/edges, two real P1 origins and frozen bounds.
    const root = fs.realpathSync.native(
      fs.mkdtempSync(join(tmpdir(), 'devup-p2-growth-')),
    )
    try {
      const declared = graph(shape, levels)
      expect(declared.vertices).toHaveLength(declared.V)
      expect(new Set(declared.vertices).size).toBe(declared.V)
      expect(declared.edges).toHaveLength(declared.E)
      for (const [from, to] of declared.edges) {
        expect(declared.vertices).toContain(from)
        expect(declared.vertices).toContain(to)
      }
      fs.mkdirSync(join(root, 'src'))
      fs.mkdirSync(join(root, 'outside'))
      for (const vertex of declared.vertices) {
        const path = join(root, vertex)
        const imports = declared.edges
          .filter(([from]) => from === vertex)
          .map(([, to]) => {
            const request = relative(dirname(path), join(root, to)).replaceAll(
              '\\',
              '/',
            )
            return `import '${request.startsWith('.') ? request : `./${request}`}';`
          })
        fs.writeFileSync(path, [...imports, 'export {}'].join('\n'))
      }
      const files = enumerateProductionSourceFiles({
        roots: ['src'],
        cwd: root,
      })
      expect(files).toEqual(
        ['src/a.ts', 'src/b.ts'].map((name) => ({
          path: join(root, name),
          realPath: join(root, name),
        })),
      )
      const context = `${shape}-${levels}`
      const expected = declared.vertices
        .map((name) => ({
          path: join(root, name),
          realPath: join(root, name),
          context,
          id: `${context}:${relative(root, join(root, name))}`,
        }))
        .sort((a, b) => {
          for (const field of ['realPath', 'path', 'context', 'id'] as const) {
            if (a[field] < b[field]) return -1
            if (a[field] > b[field]) return 1
          }
          return 0
        })
      const starts = [root, join(root, 'src'), join(root, 'outside')]
      const H = Math.max(
        ...starts.map((start) => {
          let depth = 1
          for (let path = start; dirname(path) !== path; path = dirname(path))
            depth++
          return depth
        }),
      )
      const O = 2
      const N = declared.V + declared.E + O
      const fileBudget = 16 * N
      const filesystemBudget = 512 * N * (H + 1)
      const supplied = order === 'AB' ? files : files.toReversed()
      const evidence = {
        shape,
        levels,
        order,
        declaredV: declared.V,
        declaredE: declared.E,
      }
      console.info(
        JSON.stringify({
          event: 'growth-inventory',
          ...evidence,
          O,
          N,
          H,
          starts,
          seeds: supplied,
          expected,
          fileBudget,
          filesystemBudget,
        }),
      )
      const { collectProductionFileManifest } =
        await import('../production-file-manifest')
      let mappings = 0
      let filesystem = 0
      let crossing: GrowthBudgetExceeded | undefined
      let completed = false
      const originalDirectory = fs.readdirSync
      const originalProbe = fs.statSync
      const meter = <T extends (...args: never[]) => unknown>(
        operation: T,
      ): T =>
        new Proxy(operation, {
          apply(target, receiver, args) {
            if (++filesystem > filesystemBudget) {
              crossing = new GrowthBudgetExceeded(
                'filesystem',
                filesystem,
                filesystemBudget,
              )
              throw crossing
            }
            return Reflect.apply(target, receiver, args)
          },
        })
      const directories = spyOn(fs, 'readdirSync').mockImplementation(
        meter(originalDirectory),
      )
      try {
        const probes = spyOn(fs, 'statSync').mockImplementation(
          meter(originalProbe),
        )
        try {
          // When the unchanged collector follows actual source imports under forwarded FS meters.
          const result = await collectProductionFileManifest({
            contexts: [
              {
                key: context,
                files: supplied,
                resolverOptions: { cwd: root },
                toId: (path) => {
                  if (++mappings > fileBudget) {
                    crossing = new GrowthBudgetExceeded(
                      'file',
                      mappings,
                      fileBudget,
                    )
                    throw crossing
                  }
                  return `${context}:${relative(root, path)}`
                },
              },
            ],
          })
          // Then the complete independent membership survives within graph-sized work.
          expect(result).toEqual(expected)
          expect(mappings).toBeLessThanOrEqual(fileBudget)
          expect(filesystem).toBeLessThanOrEqual(filesystemBudget)
          completed = true
        } finally {
          probes.mockRestore()
        }
      } finally {
        directories.mockRestore()
        console.info(
          JSON.stringify({
            event: 'growth-outcome',
            ...evidence,
            completed,
            mappings,
            filesystem,
            fileBudget,
            filesystemBudget,
            crossing,
            restored:
              fs.readdirSync === originalDirectory &&
              fs.statSync === originalProbe,
          }),
        )
      }
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  },
)
