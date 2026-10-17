import { isDeepStrictEqual } from 'node:util'

import {
  compareMdxBindingValues,
  snapshotMdxBindingValue,
} from './mdx-binding-value'
import type { MdxPipeline } from './mdx-pipeline'

/** Clone-value metadata (including RegExp) AND ordered compiler descriptors. */
export function equalMdxIsolatePipeline(left: MdxPipeline, right: MdxPipeline) {
  return (
    isDeepStrictEqual(left, right) &&
    !compareMdxBindingValues(
      snapshotMdxBindingValue(left.loaders),
      snapshotMdxBindingValue(right.loaders),
    )
  )
}
