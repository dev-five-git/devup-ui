import assert from 'node:assert/strict'

import { pkg } from './corpus/extraction.mjs'
import {
  barrel,
  diagnostic,
  rejected,
  source,
} from './entries-resolver-fixtures.mjs'

export async function runResolverPublication(h) {
  const { w } = h
  for (const mode of ['codeExtract', 'codeExtractWithoutSourceMap'])
    for (const precedence of [false, true])
      await h.test(`resolver-publication-${mode}-${precedence}`, () => {
        // Given: both a prior same-source sheet and unrelated styles are nonempty.
        h.read(
          h.open({
            mode,
            file: 'src/publish.tsx',
            code: `import {Box} from '${pkg}'; const x=<Box bg='blue'/>;`,
          }),
        )
        h.read(
          h.open({
            mode,
            file: 'src/other.tsx',
            code: `import {Box} from '${pkg}'; const x=<Box color='green'/>;`,
          }),
        )
        const code = precedence
          ? source.replace("bg='red'", 'styleOrder={0}')
          : source
        if (precedence) {
          w.setModuleResolver(() => null)
          const extractorError = rejected(h, () =>
            h.open({ mode, code, file: 'src/publish.tsx' }),
          )
          assert.ok(
            extractorError.includes('an integer from 1 to 254'),
            extractorError,
          )
        }
        const before = w.exportSheet()
        let calls = 0
        w.setModuleResolver(() => {
          calls++
          throw 'publication failure'
        })
        // When
        const error = rejected(h, () =>
          h.open({ mode, code, file: 'src/publish.tsx' }),
        )
        // Then
        assert.ok(calls > 0)
        assert.equal(
          error,
          diagnostic(
            'src/publish.tsx',
            2,
            23,
            './color',
            'src/publish.tsx',
            'publication failure',
          ),
        )
        assert.equal(w.exportSheet(), before)
        return { error, sheet: before, calls }
      })
  await h.test('resolver-reservation-suppression-all-roots', () => {
    // Given
    h.read(h.open())
    const before = w.exportSheet(),
      events = []
    w.setModuleResolver((specifier, importer) => {
      events.push([specifier, importer])
      throw 'reservation failure'
    })
    // When
    w.reserveClassSlots(
      [
        ['src/root-a.tsx', source, 'df'],
        ['src/root-b.tsx', source, 'df'],
      ],
      pkg,
      false,
      false,
      false,
      {},
    )
    // Then
    assert.equal(w.exportSheet(), before)
    assert.deepEqual([...new Set(events.map((event) => event[1]))].sort(), [
      'src/root-a.tsx',
      'src/root-b.tsx',
    ])
    const error = rejected(h, () =>
      w.hasDevupUI('src/later.tsx', barrel, pkg, {}),
    )
    assert.equal(
      error,
      diagnostic(
        'src/later.tsx',
        1,
        19,
        './barrel',
        'src/later.tsx',
        'reservation failure',
      ),
    )
    return { events, error }
  })
  await h.test('shorthand-invalid-target-atomic', () => {
    // Given
    w.registerShorthands({
      custom: ['marginRight', 'WebkitMaskImage', '--My-color', 'px'],
    })
    const code = `import {Box} from '${pkg}'; const x=<Box custom='red'/>;`
    const before = h.read(h.open({ code }))
    const sheet = w.exportSheet()
    // When
    const error = rejected(h, () =>
      w.registerShorthands({ custom: ['color'], invalid: ['left', 'widht'] }),
    )
    // Then
    assert.equal(
      error,
      'shorthands["invalid"][1]: shorthand `invalid` cannot use `widht` at build time: its targets must be supported CSS properties, built-in aliases or custom properties',
    )
    const after = h.read(h.open({ code }))
    assert.equal(
      after.find(([key]) => key === 'code')[1],
      before.find(([key]) => key === 'code')[1],
    )
    const fileNum = JSON.parse(w.exportFileMap())['src/entries.tsx']
    const css = w.getCss(fileNum, false)
    for (const property of [
      'margin-right',
      '-webkit-mask-image',
      '--My-color',
      'padding-left',
      'padding-right',
    ])
      assert.ok(css.includes(`${property}:red`), css)
    assert.equal(w.exportSheet(), sheet)
    h.schema(
      () => w.registerShorthands({ custom: [4] }),
      'registerShorthands(): argument shorthands must be a shorthand mapping',
    )
    return { error, before, after, css }
  })
}
