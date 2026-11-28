/**
 * The module Metro loads as `babelTransformerPath`: `withDevupUI()` points the
 * config here, and the settings it resolved reach every worker through the
 * environment.
 */
import { type BabelTransformerArgs, createTransformer } from './transform'

const transformer = createTransformer()

export function transform(args: BabelTransformerArgs): unknown {
  return transformer.transform(args)
}

export function getCacheKey(...args: unknown[]): string {
  return transformer.getCacheKey(...args)
}
