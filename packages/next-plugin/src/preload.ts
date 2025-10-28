import { readFileSync, writeFileSync } from 'node:fs'
import { basename, join, relative } from 'node:path'

import { codeExtract, registerTheme } from '@devup-ui/wasm'
import { globSync } from 'glob'

export function preload(
  excludeRegex: RegExp,
  libPackage: string,
  singleCss: boolean,
  theme: object,
  cssDir: string,
) {
  const projectRoot = process.cwd()

  const collected = globSync(['**/*.tsx', '**/*.ts', '**/*.js', '**/*.mjs'], {
    follow: true,
  })
  registerTheme(theme)
  for (const file of collected) {
    if (
      /\.(test(-d)?|d|spec)\.(tsx|ts|js|mjs)$/.test(file) ||
      /^(out|.next)[/\\]/.test(file) ||
      excludeRegex.test(file)
    )
      continue
    const filePath = relative(process.cwd(), join(projectRoot, file))
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
}
