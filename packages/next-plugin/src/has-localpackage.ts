import { readFileSync } from 'node:fs'
import { join } from 'node:path'

/**
 * has local package
 *
 * Check if the include workspace:* package is a local package
 * @returns
 */
export function hasLocalPackage() {
  const packageJson = readFileSync(join(process.cwd(), 'package.json'), 'utf-8')
  const packageJsonObject = JSON.parse(packageJson)
  return Object.values(packageJsonObject.dependencies ?? {}).some(
    (pkg: unknown) => typeof pkg === 'string' && pkg.includes('workspace:'),
  )
}
