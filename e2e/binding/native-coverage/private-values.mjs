import assert from 'node:assert/strict'

export function runValues(w, check, file) {
  const sheet = (s, extra = {}) => ({
    properties: {
      [file]: {
        0: { 0: [{ c: 'private', p: 'color', v: 'red', s, ...extra }] },
      },
    },
  })
  for (const kind of ['Media', { Media: null }, { Media: undefined }]) {
    check(`unit-enum-${JSON.stringify(kind)}`, () => {
      w.importSheet(
        sheet(
          { At: { kind, query: '(width > 1px)', selector: null } },
          { t: true, h: false },
        ),
      )
      assert.match(w.getCss(undefined, false), /red/)
    })
  }
  for (const s of [
    true,
    1,
    null,
    new Uint8Array([1]),
    { At: { kind: { Media: 1 }, query: '' } },
  ]) {
    check(`malformed-selector-${String(s)}`, () => {
      // Null is a valid absent selector; malformed inner unit payload is not.
      if (s === null) w.importSheet(sheet(s))
      else assert.throws(() => w.importSheet(sheet(s)), TypeError)
    })
  }
  for (const value of [
    1.5,
    1n,
    18446744073709551615n,
    18446744073709551616n,
    -9223372036854775809n,
    new Uint8Array([1]),
    new ArrayBuffer(1),
    new Set([1]),
    Symbol('color'),
  ]) {
    check(`malformed-color-${String(value)}`, () => {
      assert.throws(
        () => w.registerTheme({ colors: { default: { probe: value } } }),
        TypeError,
      )
    })
  }
  check('non-map-iterable-color', () => {
    const value = { [Symbol.iterator]: () => [][Symbol.iterator]() }
    assert.throws(
      () => w.registerTheme({ colors: { default: { probe: value } } }),
      TypeError,
    )
  })
  check('typed-string-rejects-bytes', () => {
    assert.throws(
      () => w.importCanonicalMap({ file: new Uint8Array([1]) }),
      TypeError,
    )
  })
  check('bool-schema-and-map-array', () => {
    assert.throws(() => w.importSheet(sheet(null, { t: 1 })), TypeError)
    w.importFileRoutes(new Map([['a', [1, 2]]]))
    w.importClassMap({ a: { zero: 0n, max: 4294967295n } })
    assert.deepEqual(JSON.parse(w.exportClassMap()), {
      a: { zero: 0, max: 4294967295 },
    })
    for (const value of [18446744073709551616n, -1n, 4294967296n])
      assert.throws(
        () => w.importClassMap({ a: { invalid: value } }),
        TypeError,
      )
  })
}
