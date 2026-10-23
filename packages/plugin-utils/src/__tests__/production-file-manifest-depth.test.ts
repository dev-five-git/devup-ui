import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { enumerateProductionSourceFiles } from '../production-source-files'

class DepthBudgetExceeded extends RangeError {
  readonly name = 'DepthBudgetExceeded'
  constructor(
    readonly meter: 'file' | 'filesystem',
    readonly count: number,
    readonly budget: number,
  ) {
    super(`${meter} depth work exceeded: ${count} > ${budget}`)
  }
}

it.each(['AB', 'BA'] as const)(
  'closes a flat 2048-edge real chain when origins arrive %s',
  async (order) => {
    // Given a flat external chain and two real P1 origins with independent full records.
    const root = fs.realpathSync.native(
      fs.mkdtempSync(join(tmpdir(), 'devup-p2-depth-')),
    )
    try {
      const depth = 2048
      const vertices = [
        'src/a.ts',
        'src/b.ts',
        ...Array.from(
          { length: depth + 1 },
          (_, index) => `outside/${index}.js`,
        ),
      ]
      fs.mkdirSync(join(root, 'src'))
      fs.mkdirSync(join(root, 'outside'))
      for (const name of vertices) {
        const index = Number.parseInt(name.slice('outside/'.length), 10)
        const code = name.startsWith('src/')
          ? "import '../outside/0.js'"
          : index < depth
            ? `import './${index + 1}.js'`
            : 'export {}'
        fs.writeFileSync(join(root, name), code)
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
      const expected = vertices
        .map((name) => ({
          path: join(root, name),
          realPath: join(root, name),
          context: 'depth',
          id: name,
        }))
        .sort((a, b) => {
          for (const field of ['realPath', 'path', 'context', 'id'] as const) {
            if (a[field] < b[field]) return -1
            if (a[field] > b[field]) return 1
          }
          return 0
        })
      const V = depth + 3
      const E = depth + 2
      const O = 2
      const N = V + E + O
      const starts = [root, join(root, 'src'), join(root, 'outside')]
      const H = Math.max(
        ...starts.map((start) => {
          let height = 1
          for (let path = start; dirname(path) !== path; path = dirname(path))
            height++
          return height
        }),
      )
      const fileBudget = 16 * N
      const filesystemBudget = 512 * N * (H + 1)
      console.info(
        JSON.stringify({
          event: 'depth-inventory',
          order,
          V,
          E,
          O,
          N,
          H,
          starts,
          expected,
          fileBudget,
          filesystemBudget,
        }),
      )
      let mappings = 0
      let filesystem = 0
      let completed = false
      const originalDirectory = fs.readdirSync
      const originalProbe = fs.statSync
      const meter = <T extends (...args: never[]) => unknown>(
        operation: T,
      ): T =>
        new Proxy(operation, {
          apply(target, receiver, args) {
            if (++filesystem > filesystemBudget)
              throw new DepthBudgetExceeded(
                'filesystem',
                filesystem,
                filesystemBudget,
              )
            return Reflect.apply(target, receiver, args)
          },
        })
      const { collectProductionFileManifest } =
        await import('../production-file-manifest')
      const directories = spyOn(fs, 'readdirSync').mockImplementation(
        meter(originalDirectory),
      )
      try {
        const probes = spyOn(fs, 'statSync').mockImplementation(
          meter(originalProbe),
        )
        try {
          // When the actual collector follows the deep source graph under forwarded meters.
          const result = await collectProductionFileManifest({
            contexts: [
              {
                key: 'depth',
                files: order === 'AB' ? files : files.toReversed(),
                resolverOptions: { cwd: root },
                toId: (path) => {
                  if (++mappings > fileBudget)
                    throw new DepthBudgetExceeded('file', mappings, fileBudget)
                  return relative(root, path).replaceAll('\\', '/')
                },
              },
            ],
          })
          // Then the full independent union fits the same graph-sized work bounds.
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
            event: 'depth-outcome',
            order,
            completed,
            mappings,
            filesystem,
            fileBudget,
            filesystemBudget,
          }),
        )
      }
    } finally {
      fs.rmSync(root, { recursive: true, force: true })
    }
  },
  120000,
)
