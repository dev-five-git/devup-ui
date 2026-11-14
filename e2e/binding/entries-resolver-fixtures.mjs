import assert from 'node:assert/strict'

import { pkg } from './corpus/extraction.mjs'

const fix =
  'Fix: repair the module resolver; return {path:string,code:string} with a non-empty path, or null/undefined for an unresolved module.'
export const diagnostic = (file, line, column, specifier, importer, cause) =>
  `${file}:${line}:${column}: module resolver failed for \`${specifier}\` from \`${importer}\`: ${cause}\n${fix}`
export const barrel =
  "import {Box} from './barrel'; export const v=<Box p={2}/>"
export const source = `import { Box } from '${pkg}';\nimport { color } from './color';\nconst x = <Box bg='red' />;`
export const expectedObject =
  'expected an object {path:string,code:string} or null/undefined'
export const rejected = (h, action) => {
  const result = h.take(action)
  assert.equal(result.caught, true)
  assert.equal(result.error.constructor, Error)
  return result.error.message
}
