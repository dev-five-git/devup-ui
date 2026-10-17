import {
  createMdxContextAdmission,
  type MdxContextAdmission,
} from './mdx-context-admission'
import { MdxInvocationError, MdxLoaderExecutionError } from './mdx-invocation'
import { isMdxRecord } from './mdx-pipeline'
import type { MdxPrewarmStep } from './mdx-prewarm-boundary'
import mdxPrewarmLoader from './mdx-prewarm-loader'

export type MdxNormalSeam = {
  readonly ruleKey: string
  readonly compilerIndex: number
  readonly compilerPath: string
  readonly ownIndex?: number
  readonly loaderCount: number
  readonly bridge: boolean
}

type Normal = (this: unknown, ...args: readonly unknown[]) => unknown
function isNormal(value: unknown): value is Normal {
  return typeof value === 'function'
}

export function installMdxNormalCapture(
  context: object,
  seam: MdxNormalSeam,
  control: {
    readonly admission: MdxContextAdmission
    readonly steps: ReadonlyMap<number, MdxPrewarmStep>
  } = { admission: createMdxContextAdmission(), steps: new Map() },
) {
  const { admission, steps } = control
  let loaders: unknown
  let compilerExecutions = 0
  let ownExecutions = 0
  let captured: readonly unknown[] | undefined
  const stop = new Error('MDX invocation-private capture stop')
  const failure = (fact: string) =>
    new MdxInvocationError(seam.ruleKey, seam.compilerPath, fact)
  const currentPath = () => {
    const index: unknown = Reflect.get(context, 'loaderIndex')
    const slot: unknown =
      Array.isArray(loaders) && typeof index === 'number'
        ? loaders[index]
        : undefined
    return isMdxRecord(slot) && typeof slot.path === 'string'
      ? slot.path
      : seam.compilerPath
  }
  const rejectExecution = (error: unknown): never => {
    if (error instanceof MdxInvocationError) throw error
    throw new MdxLoaderExecutionError(seam.ruleKey, currentPath(), error)
  }
  Object.defineProperty(context, 'loaders', {
    enumerable: true,
    configurable: true,
    get: () => loaders,
    set(value: unknown) {
      if (!Array.isArray(value) || value.length !== seam.loaderCount)
        throw failure('original loader array')
      loaders = value
      for (let index = 0; index < value.length; index++) {
        const slot: unknown = value[index]
        if (!isMdxRecord(slot)) throw failure(`normal slot ${index}`)
        for (const phase of ['normal', 'pitch'] as const) {
          const property = Object.getOwnPropertyDescriptor(slot, phase)
          if (
            !property?.configurable ||
            !property.writable ||
            !property.enumerable
          )
            throw failure(`writable existing normal slot ${index}`)
          let normal: Normal | undefined
          Object.defineProperty(slot, phase, {
            enumerable: true,
            configurable: true,
            get: () => normal,
            set(original: unknown) {
              if (
                !isNormal(original) &&
                (phase === 'pitch' || index !== seam.compilerIndex)
              ) {
                normal = undefined
                return
              }
              if (!isNormal(original))
                throw failure(`callable original normal slot ${index}`)
              normal = function (this: unknown, ...args: readonly unknown[]) {
                if (
                  !isMdxRecord(this) ||
                  this.loaderIndex !== index ||
                  this.loaders !== loaders ||
                  slot[
                    phase === 'normal' ? 'normalExecuted' : 'pitchExecuted'
                  ] !== true
                )
                  throw failure(`original runner normal position ${index}`)
                const occurrence = {
                  loader:
                    typeof slot.path === 'string'
                      ? slot.path
                      : seam.compilerPath,
                  index,
                  phase,
                }
                return admission.run(context, occurrence, () => {
                  if (phase === 'pitch')
                    return Reflect.apply(
                      original,
                      admission.view(context, occurrence),
                      args,
                    )
                  if (index === seam.compilerIndex) {
                    compilerExecutions++
                    if (compilerExecutions !== 1)
                      throw failure('exactly one compiler execution')
                    if (seam.bridge)
                      return mdxPrewarmLoader(
                        context,
                        { step: steps.get(index), admission, occurrence },
                        args[0],
                      )
                    return Reflect.apply(
                      original,
                      admission.view(context, occurrence),
                      args,
                    )
                  }
                  if (index !== seam.ownIndex)
                    return Reflect.apply(
                      original,
                      admission.view(context, occurrence),
                      args,
                    )
                  admission.check()
                  ownExecutions++
                  if (compilerExecutions !== 1 || ownExecutions !== 1)
                    throw failure(
                      'exactly one compiler before Devup normal capture',
                    )
                  captured = args
                  throw stop
                })
              }
            },
          })
        }
      }
      const position = Object.getOwnPropertyDescriptor(context, 'loaderIndex')
      if (!position?.configurable || !position.enumerable || !position.writable)
        throw failure('writable existing runner index')
      let currentIndex: unknown = position.value
      Object.defineProperty(context, 'loaderIndex', {
        configurable: position.configurable,
        enumerable: position.enumerable,
        get: () => currentIndex,
        set(next: unknown) {
          admission.advance()
          currentIndex = next
        },
      })
    },
  })
  return {
    timeoutError: () =>
      new MdxInvocationError(
        seam.ruleKey,
        currentPath(),
        'bounded original loader completion (timeout)',
      ),
    accept(error: unknown, result: unknown): unknown {
      admission.check()
      if (seam.ownIndex !== undefined) {
        if (
          error === stop &&
          captured &&
          compilerExecutions === 1 &&
          ownExecutions === 1 &&
          isMdxRecord(result)
        )
          return { ...result, result: captured }
        if (error) rejectExecution(error)
        throw failure(
          'private-stop runner callback after Devup capture (pitch bypass)',
        )
      }
      if (error) rejectExecution(error)
      if (compilerExecutions !== 1)
        throw failure('exactly one compiler execution (pitch bypass)')
      return result
    },
  }
}
