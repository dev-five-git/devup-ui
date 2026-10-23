import { expect, it } from 'bun:test'

import {
  cacheFixture,
  extractionObserver,
  rejectionBoundary,
} from './source-cache-fixture'

it('propagates real extraction errors and does not retain the failed input', async () => {
  using state = await cacheFixture()
  using extracts = extractionObserver()
  const invalid =
    'import {css} from "@devup-ui/react"; export const invalid=css({color:window.never});'
  await expect(state.transform(invalid)).rejects.toThrow()
  await expect(state.transform(invalid)).rejects.toThrow()
  expect(extracts).toHaveBeenCalledTimes(2)
  const result = await state.transform()
  expect(result).toBeDefined()
  expect(await state.transform()).toEqual(result)
  expect(extracts).toHaveBeenCalledTimes(3)
})

it('preserves actual fulfilled values at the rejection-observation boundary', async () => {
  const result = rejectionBoundary(Promise.resolve('actual fulfilled value'))
  expect(await result()).toBe('actual fulfilled value')
})
