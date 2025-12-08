import { readFileSync } from 'node:fs'

export function getPackageName(packageJsonPath: string) {
  const packageJson = readFileSync(packageJsonPath, 'utf-8')
  const packageJsonObject = JSON.parse(packageJson)
  return packageJsonObject.name
}
