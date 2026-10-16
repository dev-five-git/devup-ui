import { isAbsolute, resolve } from 'node:path'

import { isSelectedSource } from '@devup-ui/plugin-utils'
import type { ResolvedConfig } from 'vite'

export function productionEntryInputs(
  root: string,
  environment: ResolvedConfig['environments'][string],
  mdxExtensions: readonly string[],
) {
  const build = environment.build
  const native = build.rolldownOptions?.input ?? build.rollupOptions?.input
  const input = build.lib
    ? (native ?? build.lib.entry)
    : typeof build.ssr === 'string'
      ? build.ssr
      : (native ?? environment.input ?? resolve(root, 'index.html'))
  const entries = (
    typeof input === 'string' ? [input] : Object.values(input ?? {})
  ).filter((entry): entry is string => typeof entry === 'string')
  const physicalEntries = entries.filter(
    (entry) =>
      !/[\0?#]/.test(entry) &&
      (isAbsolute(entry) || !/^[a-z][\w-]*:/i.test(entry)),
  )
  const entryClosure = !physicalEntries.some((entry) =>
    entry.toLowerCase().endsWith('.html'),
  )
  return {
    physicalEntries,
    selectedEntries: physicalEntries
      .filter((entry) => isSelectedSource(entry, mdxExtensions))
      .map((entry) => resolve(root, entry)),
    nonphysical: entryClosure
      ? []
      : entries
          .filter((entry) => !physicalEntries.includes(entry))
          .map((entry) => entry.replace(/\?.*$/s, '')),
    entryClosure,
  }
}
