import { isAbsolute, join, resolve } from 'node:path'

import {
  createModulePathResolver,
  isSelectedSource,
  type PreparationInputs,
  type ResolutionInputObserver,
} from '@devup-ui/plugin-utils'
import type { ResolvedConfig } from 'vite'

import { physicalEnvironmentEntries } from './reservation-inputs'

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
    physicalEntries: physicalEntries.filter(
      (entry) => !entry.toLowerCase().endsWith('.html'),
    ),
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

export async function productionHtmlEntries(
  config: ResolvedConfig,
  environment: ResolvedConfig['environments'][string],
  preparation: {
    readonly inputs: PreparationInputs
    readonly mdxExtensions: readonly string[]
    readonly conditions: readonly string[]
    readonly observe: ResolutionInputObserver
  },
) {
  const physical = await physicalEnvironmentEntries(
    config,
    environment,
    preparation.inputs.read,
  )
  const resolver = createModulePathResolver(
    {
      cwd: config.root,
      conditions: preparation.conditions,
      includeMdx: preparation.mdxExtensions,
      onResolutionInputs: preparation.observe,
    },
    undefined,
    preparation.inputs.collect,
  )
  const entries: string[] = []
  for (const { request, context } of physical.entries) {
    const resolved = resolver(request, join(context, '__devup_entry__'))
    if (
      resolved &&
      resolved.ignored !== true &&
      isSelectedSource(resolved.path, preparation.mdxExtensions)
    )
      entries.push(resolved.path)
  }
  return entries
}
