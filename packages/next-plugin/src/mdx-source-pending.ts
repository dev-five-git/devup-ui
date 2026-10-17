import type { MdxNativeExpectation } from './mdx-source-types'

export class MdxNativeInputPendingError extends Error {
  readonly name = 'MdxNativeInputPendingError'
  constructor(readonly expectation: MdxNativeExpectation) {
    super(
      `${expectation.filename}:1:1: ordinary source requires native upstream bytes (${expectation.reason}); needs the genuine pre-Devup loader input before CSS publication`,
    )
  }
}
