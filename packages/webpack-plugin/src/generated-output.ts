import { readFile, writeFile } from 'node:fs/promises'

import { createStateWriter } from '@devup-ui/plugin-utils'

export function createGeneratedOutputWriter() {
  return createStateWriter(async (path, content, encoding) => {
    try {
      if ((await readFile(path, 'utf-8')) === content) return
    } catch (error) {
      if (!(
        error instanceof Error &&
        'code' in error &&
        error.code === 'ENOENT'
      ))
        throw error
    }
    await (encoding
      ? writeFile(path, content, encoding)
      : writeFile(path, content))
  })
}

export function normalizeSheet(content: string): string {
  // This schema is owned by exportSheet; only the innermost property sets are unordered.
  const sheet: {
    properties: Record<
      string,
      Record<string, Record<string, Record<string, unknown>[]>>
    >
  } = JSON.parse(content)
  for (const file of Object.values(sheet.properties)) {
    for (const order of Object.values(file)) {
      for (const [level, properties] of Object.entries(order)) {
        order[level] = properties
          .map((property) => ({ property, key: JSON.stringify(property) }))
          .sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0))
          .map(({ property }) => property)
      }
    }
  }
  return JSON.stringify(sheet)
}
