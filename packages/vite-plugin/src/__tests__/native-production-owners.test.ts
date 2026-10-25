import * as fs from 'node:fs'
import { writeFile } from 'node:fs/promises'
import { join } from 'node:path'

import { ProductionNumberingError } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'

import { productionFixture } from './production-fixture'
import {
  assertRetained,
  captureHistory,
  restoreHistory,
} from './reservation-native-history'

it('rejects context and extraction before preparing a configured native owner', async () => {
  // Given a genuine resolved configuration without a prepared manifest.
  const fixture = await productionFixture()
  const activation = await fixture.activation()
  try {
    // When callers request numbering before the preparation boundary.
    expect(() => activation.context).toThrow(ProductionNumberingError)
    expect(() => activation.run('client', () => 1)).toThrow(
      ProductionNumberingError,
    )
  } finally {
    activation.dispose()
    await fixture.close()
  }
})

it('propagates failed configuration preparation and recovers at the next input', async () => {
  // Given a real invalid config and a configured native owner.
  const fixture = await productionFixture()
  await fixture.file('app/src/main.js', 'export const n=1')
  const path = await fixture.file('app/devup.json', '{')
  const activation = await fixture.activation()
  try {
    // When preparation fails and the next input supplies repaired bytes.
    await expect(activation.prepare(() => {})).rejects.toBeInstanceOf(Error)
    await writeFile(path, '{}')
    await activation.prepare(
      () => {},
      () => {},
    )
    // Then the repaired owner has an actual prepared context.
    expect(activation.context).toBe('client')
  } finally {
    activation.dispose()
    await fixture.close()
  }
})

it('reprepares a failed generation when its theme update supplies the next input', async () => {
  // Given a successful native preparation followed by an operation failure.
  const fixture = await productionFixture()
  await fixture.file('app/src/main.js', 'export const n=1')
  const activation = await fixture.activation()
  try {
    await activation.prepare(() => {})
    activation.refine({})
    const key = activation.context
    const before = activation.modules(key)
    expect(() =>
      activation.run(key, () => {
        throw new TypeError('operation failed')
      }),
    ).toThrow(TypeError)
    // When theme refresh is the next input boundary.
    await activation.updateTheme()
    // Then a fresh native manifest replaces the failed generation.
    expect(activation.modules(activation.context)).not.toBe(before)
  } finally {
    activation.dispose()
    await fixture.close()
  }
})

it('reports a native build config fault without accepting incomplete output', async () => {
  // Given invalid configuration bytes at a real native build boundary.
  const fixture = await productionFixture()
  await fixture.file('app/src/main.js', 'export const n=1')
  await fixture.file('app/devup.json', '{')
  try {
    // When the actual builder starts production preparation.
    const result = fixture.run(false)
    // Then configuration failure rejects the build rather than publishing output.
    await expect(result).rejects.toThrow()
  } finally {
    await fixture.close()
  }
})

it('keeps an unresolved opaque require in native output without guessing a compiled origin', async () => {
  // Given custom-extension JavaScript with an opaque external require.
  const fixture = await productionFixture()
  await fixture.file('app/src/main.js', 'export {value} from "./opaque.custom"')
  await fixture.file(
    'app/src/opaque.custom',
    'export const value=require("external-library").css',
  )
  try {
    // When the native compiler parses that source through its public module hooks.
    const output = await fixture.run(
      false,
      [
        {
          name: 'custom-source-owner',
          load(id) {
            if (id.endsWith('.custom'))
              return 'export const value=require("external-library").css'
          },
        },
      ],
      {
        build: {
          write: false,
          lib: { entry: join(fixture.root, 'src/main.js'), formats: ['es'] },
          rolldownOptions: { external: ['external-library'] },
        },
      },
    )
    // Then unknown origins remain opaque, not a fabricated compile-time violation.
    expect(
      output
        .filter((entry) => entry.type === 'chunk')
        .map((entry) => entry.code)
        .join(''),
    ).toContain('external-library')
  } finally {
    await fixture.close()
  }
})

it('restores absent native history and compares a first learned opaque ID', async () => {
  // Given an initially absent history and an actual persisted ID file.
  const fixture = await productionFixture()
  const before = captureHistory(fixture.root)
  const name = `${'a'.repeat(64)}.json`
  await fixture.file(`app/df/numbering/${name}`, '["\\u0000virtual-styled.js"]')
  try {
    const after = captureHistory(fixture.root)
    // When first-run retention is checked and absence is restored twice.
    assertRetained(before, after)
    restoreHistory(fixture.root, before)
    restoreHistory(fixture.root, before)
    // Then no numbering directory remains observable to a new capture.
    expect(captureHistory(fixture.root)).toEqual({ kind: 'absent' })
  } finally {
    await fixture.close()
  }
})

it('preserves a history filesystem permission fault instead of classifying it as absent', async () => {
  // Given a permission failure at the filesystem read boundary.
  const fixture = await productionFixture()
  const cause = Object.assign(new Error('history permission fault'), {
    code: 'EACCES',
  })
  const original = fs.readdirSync
  const read = spyOn(fs, 'readdirSync').mockImplementation(
    new Proxy(original, {
      apply(target, receiver, args) {
        if (args[0] === join(fixture.root, 'df/numbering')) throw cause
        return Reflect.apply(target, receiver, args)
      },
    }),
  )
  try {
    // When the history helper reads through that real filesystem boundary.
    // Then only ENOENT is classified absent; this failure propagates.
    expect(() => captureHistory(fixture.root)).toThrow(cause)
  } finally {
    read.mockRestore()
    await fixture.close()
  }
})
