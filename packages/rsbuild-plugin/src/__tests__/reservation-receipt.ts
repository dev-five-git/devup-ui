import assert from 'node:assert/strict'

export function record(
  value: unknown,
): asserts value is Record<string, unknown> {
  assert(value !== null && typeof value === 'object')
}

export function parseReceipt(value: unknown) {
  record(value)
  const {
    modules,
    css,
    count,
    reservations,
    extractions,
    events,
    entries,
    compilations,
    beforeBuild,
  } = value
  record(modules)
  assert(typeof css === 'string' && typeof count === 'number')
  assert(
    typeof entries === 'number' &&
      typeof compilations === 'number' &&
      typeof beforeBuild === 'number',
  )
  assert(
    Array.isArray(reservations) &&
      Array.isArray(extractions) &&
      Array.isArray(events),
  )
  const batches = reservations.map((batch: unknown) => {
    record(batch)
    assert(Array.isArray(batch.rows))
    const rows = batch.rows.map((row: unknown) => {
      assert(Array.isArray(row) && row.length === 3)
      assert(row.every((field: unknown) => typeof field === 'string'))
      const [filename, code, directory]: unknown[] = row
      assert(
        typeof filename === 'string' &&
          typeof code === 'string' &&
          typeof directory === 'string',
      )
      return { filename, code, directory }
    })
    return {
      rows,
      flags: batch.flags,
      count: batch.count,
      classMap: batch.classMap,
      fileMap: batch.fileMap,
    }
  })
  const outputs = extractions.map((output: unknown) => {
    record(output)
    assert(typeof output.name === 'string' && typeof output.code === 'string')
    assert(
      typeof output.before === 'number' && typeof output.after === 'number',
    )
    return {
      name: output.name,
      code: output.code,
      before: output.before,
      after: output.after,
      map: output.map,
    }
  })
  return {
    modules,
    css,
    count,
    batches,
    outputs,
    events,
    entries,
    compilations,
    beforeBuild,
    classMap: value.classMap,
    fileMap: value.fileMap,
    assets: value.assets,
  }
}

export function assertRetained(previous: unknown, current: unknown): void {
  record(previous)
  record(current)
  for (const [key, value] of Object.entries(previous)) {
    if (value !== null && typeof value === 'object')
      assertRetained(value, current[key])
    else assert.deepEqual(current[key], value)
  }
}
