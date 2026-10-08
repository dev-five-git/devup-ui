import { CoverageError } from './lcov'

const qualifiedRevisions: ReadonlyMap<string, string> = new Map([
  ['1.4.0', '34cbb9a40b4bd1bd767d134a7065e66c2432a676'],
  ['1.4.2', '744846f844374847c902b5e7fd59b4342a51ef99'],
])
const runtimeIdentity = Object.freeze({
  bun: Bun.version,
  revision: Bun.revision,
})

export interface ProfileRange {
  readonly startOffset: number
  readonly endOffset: number
  readonly count: number
}
export interface ProfileFunction {
  readonly ranges: readonly ProfileRange[]
  readonly isBlockCoverage: boolean
}
export interface ProfileScript {
  readonly scriptId: string
  readonly url: string
  readonly functions: readonly ProfileFunction[]
}
export interface BasicBlock extends ProfileRange {
  readonly hasExecuted: boolean
}
export interface ScriptCapture {
  readonly profile: ProfileScript | undefined
  readonly scriptId: string
  readonly url: string
  readonly scriptType: string
  readonly sourceMapURL: string
  readonly code: string
  readonly blocks: readonly BasicBlock[]
}

export function parseProducerIdentity(
  value: unknown,
  expected: { readonly pid: number | undefined; readonly config: string },
): {
  readonly pid: number
  readonly config: string
  readonly bun: string
  readonly revision: string
  readonly profiles: readonly ProfileScript[]
} {
  const record = object(value)
  const pid = integer(record['pid'])
  const config = text(record['config'])
  const bun = text(record['bun'])
  const revision = text(record['revision'])
  if (
    qualifiedRevisions.get(bun) !== revision ||
    bun !== runtimeIdentity.bun ||
    revision !== runtimeIdentity.revision ||
    config !== expected.config ||
    pid !== expected.pid
  )
    throw new CoverageError(
      'unknown/stale producer process or configuration identity',
    )
  return Object.freeze({
    pid,
    config,
    bun,
    revision,
    profiles: parseProfiles(record['profiles']),
  })
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function object(value: unknown): Record<string, unknown> {
  if (!isObject(value)) throw new CoverageError('producer expected object')
  return value
}
export function text(value: unknown): string {
  if (typeof value !== 'string')
    throw new CoverageError('producer expected string')
  return value
}
export function integer(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0)
    throw new CoverageError('producer expected nonnegative safe integer')
  return value
}
export function boolean(value: unknown): boolean {
  if (typeof value !== 'boolean')
    throw new CoverageError('producer expected boolean')
  return value
}
function signedInteger(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value))
    throw new CoverageError('producer expected safe offset')
  return value
}
export function array(value: unknown): unknown[] {
  if (!Array.isArray(value)) throw new CoverageError('producer expected array')
  return value
}
export function parseProfiles(value: unknown): readonly ProfileScript[] {
  return Object.freeze(
    array(value).map((item) => {
      const script = object(item)
      return Object.freeze({
        scriptId: text(script['scriptId']),
        url: text(script['url']),
        functions: Object.freeze(
          array(script['functions']).map((item) => {
            const fn = object(item)
            if (text(fn['functionName']) !== '')
              throw new CoverageError(
                'unsupported native function naming profile',
              )
            return Object.freeze({
              isBlockCoverage: boolean(fn['isBlockCoverage']),
              ranges: Object.freeze(
                array(fn['ranges']).map((item) => {
                  const range = object(item)
                  return Object.freeze({
                    startOffset: integer(range['startOffset']),
                    endOffset: integer(range['endOffset']),
                    count: integer(range['count']),
                  })
                }),
              ),
            })
          }),
        ),
      })
    }),
  )
}
export function parseBlocks(value: unknown): readonly BasicBlock[] {
  return Object.freeze(
    array(value).map((item) => {
      const block = object(item)
      return Object.freeze({
        startOffset: signedInteger(block['startOffset']),
        endOffset: signedInteger(block['endOffset']),
        count: integer(block['executionCount']),
        hasExecuted: boolean(block['hasExecuted']),
      })
    }),
  )
}
