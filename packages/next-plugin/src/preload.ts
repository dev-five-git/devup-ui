import { globSync, readFileSync, writeFileSync } from 'node:fs'
import { basename, join, relative } from 'node:path'

import { codeExtract, registerTheme } from '@devup-ui/wasm'

import { findRoot } from './find-root'

export function preload(
  excludeRegex: RegExp,
  libPackage: string,
  singleCss: boolean,
  theme: object | undefined,
  cssDir: string,
) {
  const projectRoot = findRoot(process.cwd())

  const collected = globSync(['**/*.tsx', '**/*.ts', '**/*.js', '**/*.mjs'], {
    cwd: projectRoot,
    exclude: (filename) => excludeRegex.test(filename),
  })
  if (theme) registerTheme(theme)
  for (const file of collected) {
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
