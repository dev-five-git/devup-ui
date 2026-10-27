import { expect, it } from 'bun:test'
import { build } from 'vite'

import { createCompiledGuard } from '../compiled-guard'
import { DevupUI } from '../plugin'
import { nativeCompletions, productionFixture } from './production-fixture'

it('keeps native output when a plugin no longer resolves a previously loaded forwarding request', async () => {
  // Given a public resolver whose virtual request is available only on first load.
  const fixture = await productionFixture()
  const entry = await fixture.file(
    'app/src/main.custom',
    'import {value} from "ephemeral-owner"; export const result=value',
  )
  let resolutions = 0
  try {
    // When the actual bundler loads the module before the guard rechecks origins.
    const result = await build({
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      build: { write: false, lib: { entry, formats: ['es'] } },
      plugins: [
        {
          name: 'ephemeral-owner',
          resolveId(id) {
            if (id === 'ephemeral-owner' && resolutions++ === 0)
              return '\0ephemeral-owner'
          },
          load(id) {
            if (id === entry)
              return 'import {value} from "ephemeral-owner"; export const result=value'
            if (id === '\0ephemeral-owner') return 'export const value=42'
          },
        },
        createCompiledGuard({
          package: '@devup-ui/react',
          mdxExtensions: [],
          importAliases: {},
          extractCss: true,
        }),
      ],
    })
    // Then absent forwarding authority is not mistaken for a compile-time export.
    expect(resolutions).toBeGreaterThan(1)
    const outputs = (Array.isArray(result) ? result : [result]).flatMap(
      (bundle) => {
        if (!('output' in bundle))
          throw new TypeError('Expected completed native output')
        return bundle.output
      },
    )
    expect(
      outputs
        .filter((output) => output.type === 'chunk')
        .map((output) => output.code)
        .join(''),
    ).toContain('42')
  } finally {
    await fixture.close()
  }
})

it('observes nested missing source ancestors before native watch rejects the import', async () => {
  // Given an actual watch build whose dependency has multiple missing parents.
  const fixture = await productionFixture()
  const entry = await fixture.file(
    'app/src/main.js',
    'import {value} from "./absent/nested/value.js"; export {value}',
  )
  const result = await build({
    root: fixture.root,
    configFile: false,
    logLevel: 'silent',
    plugins: [DevupUI()],
    build: { watch: {}, write: false, lib: { entry, formats: ['es'] } },
  })
  if (Array.isArray(result) || !('on' in result))
    throw new TypeError('Expected native watch build')
  const next = nativeCompletions(result)
  try {
    // When the real watch compiler attempts its initial build.
    // Then the build reports the real missing request rather than emitting a bundle.
    await expect(next()).rejects.toThrow('absent/nested/value.js')
  } finally {
    await result.close()
    await fixture.close()
  }
}, 30000)
