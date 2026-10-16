import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { isMdxRecord } from './mdx-pipeline'
import { recognizeMdxPrewarmStep } from './mdx-prewarm-boundary'

const receipts = new WeakMap<
  object,
  {
    readonly root: string
    readonly module: string
    readonly sourceMap: boolean
  }
>()

/** Receiving-context producer only; neither config omission nor CLI phase is evidence. */
export function captureMdxTurboContext(loader: string, receiving: object) {
  const root: unknown = Object.getOwnPropertyDescriptor(
    receiving,
    'rootContext',
  )?.value
  const sourceMap: unknown = Object.getOwnPropertyDescriptor(
    receiving,
    'sourceMap',
  )?.value
  const module: unknown = Object.getOwnPropertyDescriptor(
    receiving,
    '_module',
  )?.value
  const step = recognizeMdxPrewarmStep(loader)
  if (
    !step ||
    typeof root !== 'string' ||
    typeof sourceMap !== 'boolean' ||
    !isMdxRecord(module) ||
    Object.getOwnPropertyDescriptor(module, '__reserved')?.value !==
      'TurbopackContext' ||
    '_compiler' in receiving ||
    'mode' in receiving
  )
    throw new TypeError(
      'supported receiving Turbopack MDX context is unavailable',
    )
  const installed = createRequire(join(root, 'package.json'))
  const metadata: unknown = JSON.parse(
    readFileSync(installed.resolve('next/package.json'), 'utf8'),
  )
  if (!isMdxRecord(metadata) || metadata.version !== '16.3.6')
    throw new TypeError('receiving Turbopack MDX requires Next 16.3.6')
  const native: object = Object.freeze(Object.create(null))
  receipts.set(native, { root, module: step.module, sourceMap })
  return Object.freeze({ native, sourceMap })
}

export function provenMdxTurboCompiler(
  context: {
    readonly native?: object
    readonly compiler?: object
    readonly mode?: string
    readonly sourceMap?: boolean
  },
  root: string,
  module: string,
) {
  const receipt = context.native && receipts.get(context.native)
  return receipt &&
    receipt.root === root &&
    receipt.module === module &&
    receipt.sourceMap === context.sourceMap &&
    context.mode === undefined &&
    context.compiler === undefined
    ? null
    : undefined
}
