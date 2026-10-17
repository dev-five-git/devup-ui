import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { describe, expect, it } from 'bun:test'

import {
  detectFinalConfigAdapter,
  supportsFinalConfigAdapter,
} from '../adapter-capability'
import { adapterFixture } from './adapter-fixture'

const landing = resolve(import.meta.dir, '../../../../apps/landing')
const require = createRequire(join(landing, 'package.json'))
const schema: unknown = require('next/dist/server/config-schema')
const source = readFileSync(require.resolve('next/dist/server/config'), 'utf8')

describe('Next final-config capability', () => {
  it('admits the actual installed schema and awaited lifecycle', () => {
    // Given / When / Then: the project's installed Next, not a version fixture.
    expect(detectFinalConfigAdapter(landing)).toBe(true)
  })

  it.each([
    null,
    {},
    { configSchema: null },
    { configSchema: {} },
    { configSchema: { safeParse: () => null } },
    { configSchema: { safeParse: () => ({ success: false }) } },
    { configSchema: { safeParse: () => ({ success: true, data: {} }) } },
    {
      configSchema: {
        safeParse: () => ({ success: true, data: { adapterPath: 1 } }),
      },
    },
    {
      configSchema: {
        safeParse: () => ({
          success: true,
          data: { experimental: { adapterPath: 'probe' } },
        }),
      },
    },
  ])('rejects old/partial schema %j', (partial) => {
    // Given: absent top-level admission. When / Then: reject even with modern hook code.
    expect(supportsFinalConfigAdapter(partial, source)).toBe(false)
  })

  it.each([
    '',
    source.replace(
      'async function applyModifyConfig',
      'function applyModifyConfig',
    ),
    source.replace(
      'if (config.adapterPath)',
      'if (config.experimental.adapterPath)',
    ),
    source.replace(
      "typeof adapterMod.modifyConfig === 'function'",
      "typeof adapterMod.modifyConfig === 'object'",
    ),
    source.replace(
      'config = await adapterMod.modifyConfig',
      'config = adapterMod.modifyConfig',
    ),
    source.replaceAll('return config;', 'return {};'),
    source.replaceAll(
      'finalizeConfig(await applyModifyConfig(',
      'finalizeConfig(applyModifyConfig(',
    ),
    `// ${source.replaceAll('\n', '\n// ')}`,
  ])('rejects partial lifecycle %#', (partial) => {
    // Given: schema admission alone. When / Then: require awaited hook and finalization.
    expect(supportsFinalConfigAdapter(schema, partial)).toBe(false)
  })

  it('falls back when Next is not installed at the captured project', () => {
    const fixture = adapterFixture()
    try {
      expect(detectFinalConfigAdapter(fixture.session.projectDir)).toBe(false)
    } finally {
      fixture.close()
    }
  })

  it('does not hide a broken installed schema module', () => {
    const fixture = adapterFixture()
    try {
      const server = join(
        fixture.session.projectDir,
        'node_modules/next/dist/server',
      )
      mkdirSync(server, { recursive: true })
      writeFileSync(
        join(server, 'config-schema.js'),
        'throw new SyntaxError("broken schema")',
      )
      expect(() =>
        detectFinalConfigAdapter(fixture.session.projectDir),
      ).toThrow(SyntaxError)
    } finally {
      fixture.close()
    }
  })
})
