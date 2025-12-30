import { readFileSync, realpathSync, writeFileSync } from 'node:fs'
import { basename, dirname, join, relative } from 'node:path'

import { codeExtract, getCss } from '@devup-ui/wasm'
import { globSync } from 'tinyglobby'

import { findTopPackageRoot } from './find-top-package-root'
import { getPackageName } from './get-package-name'
import { hasLocalPackage } from './has-localpackage'

export function preload(
  excludeRegex: RegExp,
  libPackage: string,
  singleCss: boolean,
  cssDir: string,
  include: string[],
  pwd = process.cwd(),
  nested = false,
) {
  if (include.length > 0 && hasLocalPackage()) {
    const packageRoot = findTopPackageRoot()
    const collected = globSync(['package.json', '!**/node_modules/**'], {
      followSymbolicLinks: true,
      absolute: true,
      cwd: packageRoot,
    })
      .filter((file) => include.includes(getPackageName(file)))
      .map((file) => dirname(file))

    for (const file of collected) {
      preload(excludeRegex, libPackage, singleCss, cssDir, include, file, true)
    }
    if (nested) return
  }
  const collected = globSync(['**/*.tsx', '**/*.ts', '**/*.js', '**/*.mjs'], {
    followSymbolicLinks: true,
    absolute: true,
    cwd: pwd,
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
