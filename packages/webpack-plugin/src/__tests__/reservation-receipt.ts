import assert from 'node:assert/strict'

function record(value: unknown): Readonly<Record<string, unknown>> {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value))
  const entries: readonly (readonly [string, unknown])[] = Object.entries(value)
  return Object.fromEntries(entries)
}

function strings(value: unknown): Readonly<Record<string, string>> {
  return Object.fromEntries(
    Object.entries(record(value)).map(([key, text]) => {
      assert(typeof text === 'string')
      return [key, text] as const
    }),
  )
}

function number(value: unknown): number {
  assert(typeof value === 'number')
  return value
}

function snapshot(value: unknown) {
  const fields = record(value)
  assert(typeof fields.css === 'string')
  return {
    raw: fields,
    css: fields.css,
    transformed: strings(fields.transformed),
    assets: strings(fields.assets),
    canonical: strings(fields.canonical),
    files: record(fields.files),
    residual: number(fields.residual),
  }
}

export function parseReservationReceipt(text: string) {
  const parsed: unknown = JSON.parse(text)
  const fields = record(parsed)
  assert(Array.isArray(fields.reservations) && Array.isArray(fields.extracts))
  assert(Array.isArray(fields.events))
  assert(Array.isArray(fields.atomThresholds))
  return {
    final: snapshot(fields.final),
    initial:
      fields.initial === undefined ? undefined : snapshot(fields.initial),
    compilations: number(fields.compilations),
    entryCalls: number(fields.entryCalls),
    atomThresholds: fields.atomThresholds.map((value: unknown) =>
      value === null ? undefined : number(value),
    ),
    events: fields.events.map((value: unknown) => {
      assert(typeof value === 'string')
      return value
    }),
    reservations: fields.reservations.map((value: unknown) => {
      const batch = record(value)
      assert(Array.isArray(batch.rows))
      return {
        revision: number(batch.revision),
        rows: batch.rows.map((id: unknown) => {
          assert(typeof id === 'string')
          return id
        }),
      }
    }),
    extracts: fields.extracts.map((value: unknown) => {
      const item = record(value)
      assert(typeof item.prewarm === 'boolean')
      assert(
        typeof item.sameClasses === 'boolean' &&
          typeof item.sameFiles === 'boolean',
      )
      return {
        prewarm: item.prewarm,
        sameClasses: item.sameClasses,
        sameFiles: item.sameFiles,
        residual: number(item.residual),
      }
    }),
  }
}
