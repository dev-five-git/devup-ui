import { execFile } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { promisify } from 'node:util'

import { expect, it } from 'bun:test'

const execute = promisify(execFile)

it('blocks real webpack modules while preparation is pending and shares compiled MDX/source/CSS work', async () => {
  // Given
  const repo = resolve(import.meta.dir, '../../../..')
  const source = resolve(import.meta.dir, '..')
  const artifacts = mkdtempSync(join(tmpdir(), 'devup-webpack-proof-'))
  const entry = join(artifacts, 'entry.ts')
  writeFileSync(
    entry,
    [
      `export {createWebpackCoordinatorBridge} from ${JSON.stringify(join(source, 'webpack-coordinator.ts'))}`,
      `export {startCoordinator} from ${JSON.stringify(join(source, 'coordinator.ts'))}`,
      `export {createAppContext,createSession} from ${JSON.stringify(join(source, 'session.ts'))}`,
      `export {createWasm} from ${JSON.stringify(join(source, 'wasm.ts'))}`,
    ].join('\n'),
  )
  const runtime = join(artifacts, 'runtime.cjs')
  const loader = join(artifacts, 'loader.cjs')
  const css = join(artifacts, 'css.cjs')
  try {
    for (const [input, output] of [
      [entry, runtime],
      [join(source, 'loader.ts'), loader],
      [join(source, 'css-loader.ts'), css],
    ] as const) {
      await execute(
        process.execPath,
        [
          'build',
          '--target',
          'node',
          '--packages',
          'external',
          '--format',
          'cjs',
          input,
          '--outfile',
          output,
        ],
        { cwd: repo, timeout: 30000 },
      )
    }
    // When
    const { stdout } = await execute(
      'node',
      [
        join(import.meta.dir, 'webpack-bridge-capture.cjs'),
        repo,
        artifacts,
        runtime,
        loader,
        css,
      ],
      {
        cwd: repo,
        timeout: 30000,
        env: {
          ...process.env,
          NODE_PATH: join(repo, 'packages/next-plugin/node_modules'),
        },
        encoding: 'utf8',
      },
    )
    // Then
    expect(JSON.parse(stdout)).toEqual({
      controlledPending: true,
      compiledMdx: true,
      nativeCss: 3,
      engineCount: 1,
      extractions: ['entry.js', 'page.mdx'],
      callbacks: ['server', 'client', 'client'],
      originalLocatedFailure: true,
      childCloseRetainsOwner: true,
    })
  } finally {
    rmSync(artifacts, { recursive: true, force: true })
  }
}, 60000)
