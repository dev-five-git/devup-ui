import { strict as assert } from 'node:assert'
import { readFile } from 'node:fs'

import type { MdxInvocation } from '../mdx-invocation'
import { isMdxRecord } from '../mdx-pipeline'
import type { RunLoaders } from '../mdx-prepare-runner'

type Normal = (this: unknown, ...args: readonly unknown[]) => unknown
function isNormal(value: unknown): value is Normal {
  return typeof value === 'function'
}

export async function nativeOwnControl(
  invocation: MdxInvocation,
  runner: {
    readonly resource: string
    readonly context: object
    readonly runLoaders: RunLoaders
  },
  ownNormal: Normal,
) {
  let assigned: unknown
  let nativeInput: readonly unknown[] | undefined
  let compiles = 0
  Object.defineProperty(runner.context, 'loaders', {
    enumerable: true,
    configurable: true,
    get: () => assigned,
    set(value: unknown) {
      assert.ok(Array.isArray(value))
      assigned = value
      for (const index of [invocation.ownIndex, invocation.compilerIndex]) {
        const slot: unknown = value[index]
        assert.ok(isMdxRecord(slot))
        let normal: unknown
        Object.defineProperty(slot, 'normal', {
          enumerable: true,
          configurable: true,
          get: () => normal,
          set(original: unknown) {
            assert.ok(isNormal(original))
            if (index === invocation.ownIndex) assert.equal(original, ownNormal)
            normal = function (this: unknown, ...args: readonly unknown[]) {
              if (index === invocation.ownIndex) nativeInput = args
              else compiles++
              return Reflect.apply(original, this, args)
            }
          },
        })
      }
    },
  })
  await new Promise<void>((done, reject) =>
    runner.runLoaders(
      {
        resource: runner.resource,
        loaders: invocation.loaders,
        context: runner.context,
        readResource: readFile,
      },
      (error) => (error ? reject(error) : done()),
    ),
  )
  assert.equal(compiles, 1)
  assert.ok(nativeInput)
  return nativeInput
}
