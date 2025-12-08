import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'

/**
 * find package root
 *
 * Find the root of the package by checking the package.json file
 * @returns
 */
export function findTopPackageRoot(pwd = process.cwd()) {
  let current = pwd
  let topWithPackage: string | null = null

  while (true) {
    if (existsSync(join(current, 'package.json'))) {
      topWithPackage = current
    }

    const parent = dirname(current)
    if (parent === current) {
      break
    }
    current = parent
  }

  return topWithPackage ?? pwd
}
