import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import {
  createThemeInterfaceArgs,
  type DevupEngine,
  loadCompiler,
  loadDevupConfigSync,
} from '@devup-ui/plugin-utils'

/**
 * `native.d.ts`: it imports the declarations that type each component of
 * `@devup-ui/react` as the React Native component it renders
 */
export const NATIVE_TYPES = "import '@devup-ui/react/native/types'\n"

/** What the types of a project are written from */
export interface TypesOptions {
  /** The directory written, absolute */
  readonly distDir: string
  /** devup.json, absolute */
  readonly devupFile: string
  /** The package Devup UI is imported from */
  readonly package: string
  readonly engine?: DevupEngine
}

/**
 * Write the TypeScript declarations of the project into `distDir`, as the web
 * plugins do: `theme.d.ts`, the tokens of devup.json, and `native.d.ts`, which
 * types the components for React Native. The directory ignores itself in git.
 */
export function writeTypes(options: TypesOptions): void {
  const compiler = loadCompiler(options.engine)
  compiler.registerTheme(loadDevupConfigSync(options.devupFile).theme ?? {})
  mkdirSync(options.distDir, { recursive: true })
  writeFileSync(join(options.distDir, '.gitignore'), '*', 'utf-8')
  writeFileSync(
    join(options.distDir, 'theme.d.ts'),
    compiler.getThemeInterface(...createThemeInterfaceArgs(options.package)),
    'utf-8',
  )
  writeFileSync(join(options.distDir, 'native.d.ts'), NATIVE_TYPES, 'utf-8')
}
