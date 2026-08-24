import { mkdtemp, readdir, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { describe, expect, test } from 'bun:test'

const rootDir = import.meta.dir
const publishScript = resolve(rootDir, 'publish-package.ts')

async function run(args: string[], cwd: string) {
  const process = Bun.spawn(args, {
    cwd,
    stdout: 'pipe',
    stderr: 'pipe',
  })
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ])
  return { exitCode, stdout, stderr }
}

async function readPackedManifest(tarball: string) {
  const result = await run(
    ['tar', '-xOf', tarball, 'package/package.json'],
    rootDir,
  )
  expect(result.exitCode).toBe(0)
  return JSON.parse(result.stdout) as Record<string, unknown>
}

describe('trusted publish package', () => {
  test('verify-workspace packs every public package without unresolved ranges', async () => {
    const result = await run(
      ['bun', publishScript, '--verify-workspace'],
      rootDir,
    )
    expect(result.exitCode).toBe(0)
    expect(result.stdout.trim().split(/\r?\n/)).toEqual([
      'verified bindings/devup-ui-wasm',
      'verified packages/bun-plugin',
      'verified packages/components',
      'verified packages/eslint-plugin',
      'verified packages/next-plugin',
      'verified packages/plugin-utils',
      'verified packages/react',
      'verified packages/reset-css',
      'verified packages/rsbuild-plugin',
      'verified packages/vite-plugin',
      'verified packages/webpack-plugin',
    ])
  })

  test('pack-only works from the bindings package directory', async () => {
    const destination = await mkdtemp(join(tmpdir(), 'devup-ui-bindings-pack-'))
    try {
      const result = await run(
        ['bun', publishScript, '--pack-only', destination],
        resolve(rootDir, 'bindings/devup-ui-wasm'),
      )
      expect(result.exitCode).toBe(0)
      expect(
        (await readdir(destination)).filter((file) => file.endsWith('.tgz')),
      ).toHaveLength(1)
    } finally {
      await rm(destination, { recursive: true, force: true })
    }
  })

  test('pack-only materializes workspace ranges from current package versions', async () => {
    const destination = await mkdtemp(join(tmpdir(), 'devup-ui-pack-test-'))
    try {
      const result = await run(
        ['bun', publishScript, '--pack-only', destination],
        resolve(rootDir, 'packages/components'),
      )
      expect(result.exitCode).toBe(0)

      const tarballs = (await readdir(destination)).filter((file) =>
        file.endsWith('.tgz'),
      )
      expect(tarballs).toHaveLength(1)

      const manifest = await readPackedManifest(
        join(destination, tarballs[0] as string),
      )
      const reactManifest = JSON.parse(
        await readFile(resolve(rootDir, 'packages/react/package.json'), 'utf8'),
      ) as { version: string }
      const viteManifest = JSON.parse(
        await readFile(
          resolve(rootDir, 'packages/vite-plugin/package.json'),
          'utf8',
        ),
      ) as { version: string }

      expect(manifest.dependencies).toMatchObject({
        '@devup-ui/react': `^${reactManifest.version}`,
      })
      expect(manifest.devDependencies).toMatchObject({
        '@devup-ui/vite-plugin': `^${viteManifest.version}`,
      })
      expect(manifest.peerDependencies).toMatchObject({
        '@devup-ui/react': `^${reactManifest.version}`,
      })
      expect(JSON.stringify(manifest)).not.toContain('workspace:')
    } finally {
      await rm(destination, { recursive: true, force: true })
    }
  })

  test('verify-tarball rejects unresolved workspace dependencies', async () => {
    const fixtureDir = await mkdtemp(join(tmpdir(), 'devup-ui-invalid-pack-'))
    try {
      await Bun.write(
        join(fixtureDir, 'package.json'),
        JSON.stringify({
          name: 'invalid-workspace-package',
          version: '1.0.0',
          dependencies: { '@devup-ui/react': 'workspace:^' },
        }),
      )
      const packed = await run(
        ['npm', 'pack', '--json', '--pack-destination', fixtureDir],
        fixtureDir,
      )
      expect(packed.exitCode).toBe(0)
      const [{ filename }] = JSON.parse(packed.stdout) as Array<{
        filename: string
      }>

      const verified = await run(
        ['bun', publishScript, '--verify-tarball', join(fixtureDir, filename)],
        rootDir,
      )
      expect(verified.exitCode).toBe(1)
      expect(verified.stderr).toContain(
        'Unresolved workspace dependency: dependencies.@devup-ui/react',
      )
    } finally {
      await rm(fixtureDir, { recursive: true, force: true })
    }
  })
})
