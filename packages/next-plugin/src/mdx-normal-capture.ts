import { MdxInvocationError, MdxLoaderExecutionError } from './mdx-invocation'
import { isMdxRecord } from './mdx-pipeline'
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

export function installMdxNormalCapture(context: object, seam: MdxNormalSeam) {
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
      for (const index of [seam.compilerIndex, seam.ownIndex]) {
        if (index === undefined) continue
        const slot: unknown = value[index]
        if (!isMdxRecord(slot)) throw failure(`normal slot ${index}`)
        const property = Object.getOwnPropertyDescriptor(slot, 'normal')
        if (
          !property?.configurable ||
          !property.writable ||
          !property.enumerable
        )
          throw failure(`writable existing normal slot ${index}`)
        let normal: Normal | undefined
        Object.defineProperty(slot, 'normal', {
          enumerable: true,
          configurable: true,
          get: () => normal,
          set(original: unknown) {
            if (!isNormal(original))
              throw failure(`callable original normal slot ${index}`)
            normal = function (this: unknown, ...args: readonly unknown[]) {
              if (
                !isMdxRecord(this) ||
                this.loaderIndex !== index ||
                this.loaders !== loaders ||
                slot.normalExecuted !== true
              )
                throw failure(`original runner normal position ${index}`)
              if (index === seam.compilerIndex) {
                compilerExecutions++
                if (compilerExecutions !== 1)
                  throw failure('exactly one compiler execution')
                return Reflect.apply(
                  seam.bridge ? mdxPrewarmLoader : original,
                  this,
                  args,
                )
              }
              ownExecutions++
              if (compilerExecutions !== 1 || ownExecutions !== 1)
                throw failure(
                  'exactly one compiler before Devup normal capture',
                )
              captured = args
              throw stop
            }
          },
        })
      }
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
