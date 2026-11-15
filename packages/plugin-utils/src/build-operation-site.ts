import type { GenerationSite } from './build-generation'

/** Capture once for adapter setup operations that have no compiler source. */
export function buildOperationSite(operation: string): GenerationSite {
  const stack = new Error().stack ?? ''
  for (const row of stack.split('\n').slice(2)) {
    const found = row.match(/^\s*at (.+):(\d+):(\d+)(\)?)$/)
    if (found === null) continue
    let file = found[1]
    if (found[4] === ')') {
      let depth = 1
      for (let index = file.length - 1; index >= 0; index--) {
        if (file[index] === ')') depth++
        else if (file[index] === '(') depth--
        if (depth === 0) {
          file = file.slice(index + 1)
          break
        }
      }
    }
    return {
      file,
      line: Number(found[2]),
      column: Number(found[3]),
      operation,
    }
  }
  throw new Error('build operation requires an actual source callsite')
}
