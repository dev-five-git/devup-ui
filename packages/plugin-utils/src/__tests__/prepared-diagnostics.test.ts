import { expect, it } from 'bun:test'

import { preparedDiagnostics } from '../prepared-diagnostics'

it('formats label-less diagnostic fixtures and unknown diagnostic values', () => {
  expect(
    preparedDiagnostics('file.mdx', '', ['failure', { message: 'empty' }]),
  ).toBe('file.mdx:1:1: failure\nfile.mdx:1:1: empty')
})

it('uses the first compiled position when a diagnostic label has no offset', () => {
  expect(
    preparedDiagnostics('file.mdx', '', [
      { message: 'label', labels: [null, {}] },
    ]),
  ).toBe('file.mdx:1:1: label')
})
