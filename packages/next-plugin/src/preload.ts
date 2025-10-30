import { readFileSync, realpathSync, writeFileSync } from 'node:fs'
import { basename, join, relative } from 'node:path'

import { codeExtract, getCss } from '@devup-ui/wasm'
import { globSync } from 'glob'
export function preload(
  excludeRegex: RegExp,
  libPackage: string,
  singleCss: boolean,
  cssDir: string,
) {
  const collected = globSync(['**/*.tsx', '**/*.ts', '**/*.js', '**/*.mjs'], {
    follow: true,
    absolute: true,
  })
  // fix multi core build issue
  collected.sort()
  for (const file of collected) {
    const filePath = relative(process.cwd(), realpathSync(file))
    if (
      /\.(test(-d)?|d|spec)\.(tsx|ts|js|mjs)$/.test(filePath) ||
      /^(out|.next)[/\\]/.test(filePath) ||
      excludeRegex.test(filePath)
    )
      continue
    const { cssFile, css } = codeExtract(
      filePath,
      readFileSync(filePath, 'utf-8'),
      libPackage,
      cssDir,
      singleCss,
      false,
      true,
    )

    if (cssFile) {
      writeFileSync(join(cssDir, basename(cssFile!)), css ?? '', 'utf-8')
    }
  }
  writeFileSync(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8')
}
