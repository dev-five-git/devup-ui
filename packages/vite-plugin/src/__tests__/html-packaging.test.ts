import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'

it.each(['cjs', 'mjs'])(
  'awaits public parse5 discovery from the published %s artifact',
  (extension) => {
    // Given
    using project = fixture()
    project.write('devup.json', '{}')
    project.write('©.js', STYLE)
    project.write(
      'index.html',
      '<script type=module src="/&copy;.js"></script>',
    )
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
    import { resolve } from 'node:path';
    const require = createRequire(import.meta.url);
    const api = ${load};
    const root = ${JSON.stringify(project.root)};
    const [plugin] = api.DevupUI({
      singleCss: true, devupFile: resolve(root, 'devup.json'),
      distDir: resolve(root, 'df'), cssDir: resolve(root, 'df/css')
    });
    try {
      await plugin.configResolved({ command: 'build', root, publicDir: false });
      const watched = [];
      await plugin.buildStart.call({ addWatchFile(file) { watched.push(file); } });
      console.log(resolve(watched.find(file => file.endsWith('/©.js'))));
    } finally {
      await plugin.closeBundle();
    }
  `,
      ],
      { encoding: 'utf8' },
    )
    // Then
    expect(code).toMatch(/import\(["']parse5["']\)/)
    expect(result.status).toBe(0)
    expect(result.stdout.trim()).toBe(resolve(project.root, '©.js'))
  },
)
