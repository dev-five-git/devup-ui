import assert from 'node:assert/strict'
import * as fs from 'node:fs/promises'
import { join } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { spyOn } from 'bun:test'

import { lifecycle } from './plugin-lifecycle-fixture'

export const SOURCE =
  'import {Box} from "@devup-ui/react"; import {tone} from "./tone"; export const A=()=> <Box color={tone}/>'

export async function cacheFixture(prepare = true) {
  assert.equal(
    utils.getBuildOwnership().running,
    0,
    'cache schedule needs its own real engine',
  )
  const state = await lifecycle()
  assert.equal(
    utils.getBuildOwnership().running,
    1,
    'only this Vite lease may be active',
  )
  state.project.write('src/tone.ts', 'export const tone="red"')
  state.project.write('src/A.tsx', SOURCE)
  const id = join(state.project.root, 'src/A.tsx').replaceAll('\\', '/')
  const supplier = join(state.project.root, 'src/tone.ts').replaceAll('\\', '/')
  const cssFile = join(state.project.root, 'df/css/devup-ui.css')
  const context = { environment: { name: 'client' }, addWatchFile() {} }
  if (prepare) await state.plugin.buildStart.call(context)
  return {
    ...state,
    id,
    supplier,
    cssFile,
    context,
    transform(code = SOURCE, fullId = id) {
      return state.plugin.transform.call(context, code, fullId)
    },
  }
}

/** Forward the real call, including its real rejection; hold only settlement. */
export function writeGate(path: string) {
  const { writeFile } = fs
  const original = writeFile.bind(fs)
  const entered = Promise.withResolvers<void>()
  const released = Promise.withResolvers<void>()
  let armed = true
  const spy = spyOn(fs, 'writeFile').mockImplementation((...args) => {
    const result = original(...args)
    if (!armed || String(args[0]) !== path) return result
    armed = false
    return result.finally(async () => {
      entered.resolve()
      await released.promise
    })
  })
  return {
    entered: entered.promise,
    release: released.resolve,
    spy,
    [Symbol.dispose]() {
      released.resolve()
      spy.mockRestore()
    },
  }
}

export function extractionObserver() {
  const { codeExtract } = wasm
  const original = codeExtract.bind(wasm)
  return spyOn(wasm, 'codeExtract').mockImplementation((...args) =>
    original(...args),
  )
}

/** Observe now, assert later: Bun's rejects matcher eagerly waits when invoked. */
export function rejectionBoundary(work: Promise<unknown>) {
  const settled = work.then(
    (value) => () => value,
    (error: unknown) => () => {
      throw error
    },
  )
  return async () => (await settled)()
}

/** Observe the synchronous capture turn finishing, even if hit registration is omitted. */
export function captureBoundary() {
  const { getBuildOwnership } = utils
  const original = getBuildOwnership.bind(utils)
  const reached = Promise.withResolvers<void>()
  const spy = spyOn(utils, 'getBuildOwnership').mockImplementation(() => {
    const ownership = original()
    queueMicrotask(reached.resolve)
    return ownership
  })
  return {
    reached: reached.promise,
    [Symbol.dispose]() {
      spy.mockRestore()
    },
  }
}

export function transformedCode(value: unknown): string {
  assert(typeof value === 'object' && value !== null)
  assert('code' in value && typeof value.code === 'string')
  return value.code
}

export function cssLoader(value: unknown): (id: string) => string | undefined {
  assert(typeof value === 'object' && value !== null)
  assert('load' in value && typeof value.load === 'function')
  const load = value.load
  return (id) => {
    const result: unknown = load.call(value, id)
    assert(result === undefined || typeof result === 'string')
    return result
  }
}
