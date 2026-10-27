import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

it('discovers every surviving test exactly once in the two actual native config partitions', () => {
  const repository = resolve(import.meta.dir, '../../../..')
  const root = mkdtempSync(join(tmpdir(), 'devup-native-partition-'))
  const files = [
    ...new Bun.Glob('packages/**/*').scanSync({
      cwd: repository,
      onlyFiles: true,
    }),
  ]
    .map((file) => file.replaceAll('\\', '/'))
    .filter(
      (file) =>
        !file
          .split('/')
          .some(
            (part) =>
              part.startsWith('.') || part.toLowerCase() === 'node_modules',
          ),
    )
    .filter((file) =>
      /(?:\.|_)(?:test|spec)\.(?:js|jsx|ts|tsx|mjs|cjs|mts|cts)$/i.test(file),
    )
    .sort()
  try {
    for (const file of files) {
      mkdirSync(dirname(join(root, file)), { recursive: true })
      writeFileSync(
        join(root, file),
        `import {it} from 'bun:test';import {appendFileSync} from 'node:fs';it('witness',()=>appendFileSync(process.env.NATIVE_RECEIPT,${JSON.stringify(file + '\n')}))`,
      )
    }
    const witnessed: string[] = []
    for (const [name, lane] of [
      ['bunfig.toml', 'root'],
      ['bunfig.vite.toml', 'vite'],
    ] as const) {
      const config = readFileSync(join(repository, name), 'utf8').replace(
        /^preload = .*$/gm,
        '',
      )
      writeFileSync(join(root, name), config)
      const receipt = join(root, `${lane}.txt`)
      const result = spawnSync(
        process.execPath,
        [
          `--config=${join(root, name)}`,
          'test',
          ...(lane === 'root' ? ['--isolate'] : []),
        ],
        {
          cwd: root,
          encoding: 'utf8',
          env: { ...process.env, NATIVE_RECEIPT: receipt },
        },
      )
      if (result.error) throw result.error
      expect(result.status).toBe(0)
      const executed = readFileSync(receipt, 'utf8')
        .trim()
        .split(/\r?\n/)
        .sort()
      expect(executed.length).toBeGreaterThan(0)
      expect(executed).toEqual(
        files.filter(
          (file) =>
            file.startsWith('packages/vite-plugin/') === (lane === 'vite'),
        ),
      )
      witnessed.push(...executed)
    }
    expect(witnessed.sort()).toEqual(files)
    expect(new Set(witnessed).size).toBe(files.length)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
