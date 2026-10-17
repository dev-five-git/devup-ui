import { describe, expect, it } from 'bun:test'

import { locatedError } from '../build-error'

const fields = {
  file: '/app/src/page.tsx',
  what: 'devup-ui prewarm',
  code: 'prewarm',
  needs: 'a readable file',
}

describe('locatedError', () => {
  it('puts the location, the construct and what is needed in the message', () => {
    const cause = new Error('boom')

    const error = locatedError({ ...fields, cause })

    expect(error.message).toBe(
      '/app/src/page.tsx:1:1: devup-ui prewarm cannot use `prewarm` at build time: boom; needs a readable file',
    )
    expect(error.cause).toBe(cause)
  })

  it('describes a cause that is not an Error', () => {
    expect(locatedError({ ...fields, cause: 'plain' }).message).toContain(
      'at build time: plain; needs',
    )
  })

  it.each([
    '/app/src/a.tsx:3:7: Box cannot use `x` at build time: y',
    String.raw`C:\app\a.tsx:12:1: Box cannot use ...`,
  ])('keeps a cause that already says where it is: %s', (message) => {
    const cause = new Error(message)

    expect(locatedError({ ...fields, cause })).toBe(cause)
  })
})
