import { mkdirSync, symlinkSync, unlinkSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { sourceFixture } from './mdx-source-fixture'

export const paletteMdx = `import { css } from '@devup-ui/react'

import { color } from 'palette'

export const style = css({color})

# Palette`

export function paletteFixture(
  files: Record<string, string> = { 'app/page.mdx': paletteMdx },
  development = true,
) {
  const f = sourceFixture(files, {}, development)
  unlinkSync(join(f.root, 'node_modules'))
  mkdirSync(join(f.root, 'node_modules'))
  const installed = resolve(
    import.meta.dir,
    '../../../../apps/landing/node_modules',
  )
  for (const name of ['react', 'next', '@devup-ui'])
    symlinkSync(
      join(installed, name),
      join(f.root, 'node_modules', name),
      'junction',
    )
  function select(color: string) {
    return f.write(
      'node_modules/palette/package.json',
      JSON.stringify({
        name: 'palette',
        version: '1.0.0',
        exports: `./${color}.js`,
      }),
    )
  }
  select('red')
  f.write('node_modules/palette/red.js', 'export const color = "red"')
  f.write('node_modules/palette/blue.js', 'export const color = "blue"')
  return {
    ...f,
    select,
    manifest: join(f.root, 'node_modules/palette/package.json'),
  }
}
