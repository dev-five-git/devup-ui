import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

/** Fail closed for unfamiliar Next implementations, without a version floor. */
export function supportsFinalConfigAdapter(
  schemaModule: unknown,
  configSource: string,
): boolean {
  if (
    typeof schemaModule !== 'object' ||
    schemaModule === null ||
    !('configSchema' in schemaModule)
  )
    return false
  const schema = schemaModule.configSchema
  if (
    typeof schema !== 'object' ||
    schema === null ||
    !('safeParse' in schema) ||
    typeof schema.safeParse !== 'function'
  )
    return false
  const parsed: unknown = schema.safeParse({
    adapterPath: 'devup-capability-probe',
  })
  if (
    typeof parsed !== 'object' ||
    parsed === null ||
    !('success' in parsed) ||
    parsed.success !== true ||
    !('data' in parsed) ||
    typeof parsed.data !== 'object' ||
    parsed.data === null ||
    !('adapterPath' in parsed.data) ||
    parsed.data.adapterPath !== 'devup-capability-probe'
  )
    return false

  // Recognize the shipped CommonJS lifecycle, not a comment or version literal.
  const source = configSource.replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, '')
  const hook = source.match(
    /async function applyModifyConfig\([^)]*\)\s*\{([\s\S]*?)\n\}/,
  )?.[1]
  return (
    hook !== undefined &&
    /if\s*\(config\.adapterPath\)/.test(hook) &&
    /typeof adapterMod\.modifyConfig === ['"]function['"]/.test(hook) &&
    /config\s*=\s*await adapterMod\.modifyConfig\(config,\s*\{/.test(hook) &&
    /return config\s*;/.test(hook) &&
    /finalizeConfig\(await applyModifyConfig\(/.test(source)
  )
}

/** Resolve the project's installed Next; a missing/old API uses the legacy path. */
export function detectFinalConfigAdapter(projectDir: string): boolean {
  const require = createRequire(join(projectDir, 'package.json'))
  try {
    const schema: unknown = require('next/dist/server/config-schema')
    const source = readFileSync(
      require.resolve('next/dist/server/config'),
      'utf8',
    )
    return supportsFinalConfigAdapter(schema, source)
  } catch (cause) {
    if (
      cause instanceof Error &&
      'code' in cause &&
      (cause.code === 'MODULE_NOT_FOUND' || cause.code === 'ENOENT')
    )
      return false
    throw cause
  }
}
