import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each(['cjs', 'mjs'])(
  'awaits public parse5 discovery from the published %s artifact',
  (extension) => {
    // Given
    const filename = resolve(import.meta.dir, `../../dist/index.${extension}`)
    const code = readFileSync(filename, 'utf8')
    const load =
      extension === 'cjs'
        ? `require(${JSON.stringify(filename)})`
        : `await import(${JSON.stringify(`file:///${filename.replaceAll('\\', '/')}`)})`
    // When
    const result = spawnSync(
      'node',
      [
        '--input-type=module',
        '-e',
        `
    import { createRequire } from 'node:module';
    const require = createRequire(import.meta.url);
    const api = ${load};
    const entries = await api.htmlReservationEntries('<script type=module src="/&copy;.js"></script>', {
      command: 'build',
      root: process.cwd(), filename: process.cwd() + '/index.html', publicDir: false
    });
    console.log(entries[0].request);
  `,
      ],
      { encoding: 'utf8' },
    )
    // Then
    expect(code).toMatch(/import\(["']parse5["']\)/)
    expect(result.status).toBe(0)
    expect(result.stdout.trim()).toBe(resolve('©.js'))
  },
)
