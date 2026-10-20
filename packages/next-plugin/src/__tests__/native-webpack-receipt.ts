import assert from 'node:assert/strict'

import type {
  Aliases,
  ExtractArgs,
  NativeEvent,
  Payload,
  ReserveArgs,
  Row,
  SlotMap,
  Snapshot,
} from './native-webpack-types.ts'

function object(value: unknown): Readonly<Record<string, unknown>> {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value))
  const entries: readonly (readonly [string, unknown])[] = Object.entries(value)
  return Object.fromEntries(entries)
}
function list(value: unknown): readonly unknown[] {
  assert(Array.isArray(value))
  return value
}
function text(value: unknown): string {
  assert(typeof value === 'string')
  return value
}
function number(value: unknown): number {
  assert(typeof value === 'number' && Number.isSafeInteger(value) && value >= 0)
  return value
}
function boolean(value: unknown): boolean {
  assert(typeof value === 'boolean')
  return value
}
function optionalText(value: unknown): string | undefined {
  return value === undefined ? undefined : text(value)
}
function slots(value: unknown): SlotMap {
  return Object.fromEntries(
    Object.entries(object(value)).map(([key, slot]) => [key, number(slot)]),
  )
}
function aliases(value: unknown): Aliases {
  return Object.fromEntries(
    Object.entries(object(value)).map(([key, alias]) => [
      key,
      alias === null ? null : text(alias),
    ]),
  )
}
export function parseSnapshot(value: unknown): Snapshot {
  const fields = object(value)
  return {
    classes: Object.fromEntries(
      Object.entries(object(fields.classes)).map(([key, bucket]) => [
        key,
        slots(bucket),
      ]),
    ),
    files: slots(fields.files),
    residual: number(fields.residual),
    css: text(fields.css),
  }
}
function reserveArgs(value: unknown): ReserveArgs {
  const args = list(value)
  assert.equal(args.length, 6)
  const rows = list(args[0]).map((value): Row => {
    const row = list(value)
    assert.equal(row.length, 3)
    return [text(row[0]), text(row[1]), text(row[2])]
  })
  return [
    rows,
    text(args[1]),
    boolean(args[2]),
    boolean(args[3]),
    boolean(args[4]),
    aliases(args[5]),
  ]
}
function extractArgs(value: unknown): ExtractArgs {
  const args = list(value)
  assert.equal(args.length, 8)
  return [
    text(args[0]),
    text(args[1]),
    text(args[2]),
    text(args[3]),
    boolean(args[4]),
    boolean(args[5]),
    boolean(args[6]),
    aliases(args[7]),
  ]
}
function payload(value: Readonly<Record<string, unknown>>): Payload {
  switch (value.kind) {
    case 'reset':
      return { kind: 'reset', state: parseSnapshot(value.state) }
    case 'reserve':
      return {
        kind: 'reserve',
        args: reserveArgs(value.args),
        before: parseSnapshot(value.before),
        after: parseSnapshot(value.after),
      }
    case 'extract-start':
      return {
        kind: 'extract-start',
        args: extractArgs(value.args),
        before: number(value.before),
      }
    case 'extract': {
      const output = object(value.output)
      return {
        kind: 'extract',
        args: extractArgs(value.args),
        before: number(value.before),
        after: number(value.after),
        output: {
          code: text(output.code),
          css: optionalText(output.css),
          cssFile: optionalText(output.cssFile),
          map: optionalText(output.map),
          dependencies: list(output.dependencies).map(text),
          updatedBaseStyle: boolean(output.updatedBaseStyle),
        },
      }
    }
    case 'options':
      return {
        kind: 'options',
        compilerRole: text(value.compilerRole),
        dev: boolean(value.dev),
        buildId: text(value.buildId),
        delegation: number(value.delegation),
      }
    case 'compilation':
      return {
        kind: 'compilation',
        compilerName: optionalText(value.compilerName),
      }
    case 'done':
      return {
        kind: 'done',
        errors: boolean(value.errors),
        state: parseSnapshot(value.state),
      }
    default:
      throw new TypeError(`Unknown native event kind: ${String(value.kind)}`)
  }
}
export function parseEvents(contents: string): readonly NativeEvent[] {
  return contents
    .trim()
    .split('\n')
    .map((line) => {
      const parsed: unknown = JSON.parse(line)
      const value = object(parsed)
      return {
        ...payload(value),
        pid: number(value.pid),
        epoch: number(value.epoch),
        phase: text(value.phase),
        role: text(value.role),
      }
    })
}
export function parseStateReceipt(contents: string) {
  const parsed: unknown = JSON.parse(contents)
  const value = object(parsed)
  return { old: parseSnapshot(value.old), final: parseSnapshot(value.final) }
}
export function parseEdge(contents: string) {
  const parsed: unknown = JSON.parse(contents)
  const edge = text(object(parsed).edge)
  assert(edge.length > 0)
  return { edge }
}
export function parseSlots(contents: string): SlotMap {
  const parsed: unknown = JSON.parse(contents)
  return slots(parsed)
}
export function parseClasses(contents: string): Snapshot['classes'] {
  const parsed: unknown = JSON.parse(contents)
  return Object.fromEntries(
    Object.entries(object(parsed)).map(([key, bucket]) => [key, slots(bucket)]),
  )
}
