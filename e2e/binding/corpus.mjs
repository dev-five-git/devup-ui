import { boundary } from './corpus/boundary.mjs'
import { extraction } from './corpus/extraction.mjs'
import { state } from './corpus/state.mjs'

export {
  box,
  dependent,
  file,
  pkg,
  source,
  theme,
} from './corpus/extraction.mjs'

export const corpus = [...extraction, ...state, ...boundary]
