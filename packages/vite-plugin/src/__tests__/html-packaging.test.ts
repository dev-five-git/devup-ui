import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

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
    const vite = pathToFileURL(
      createRequire(import.meta.url).resolve('vite'),
    ).href
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
    const { resolveConfig } = await import(${JSON.stringify(vite)});
    const api = ${load};
    const root = ${JSON.stringify(project.root)};
    const [plugin] = api.DevupUI({
      singleCss: true, devupFile: resolve(root, 'devup.json'),
      distDir: resolve(root, 'df'), cssDir: resolve(root, 'df/css')
    });
    try {
      const config = await resolveConfig({ configFile: false, root, publicDir: false }, 'build');
      await plugin.configResolved(config);
      const watched = [];
      await plugin.buildStart.call({ addWatchFile(file) { watched.push(file); } }, { input: resolve(root, 'index.html') });
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
    expect({ status: result.status, stderr: result.stderr }).toMatchObject({
      status: 0,
    })
    expect(result.stdout.trim()).toBe(resolve(project.root, '©.js'))
  },
)
