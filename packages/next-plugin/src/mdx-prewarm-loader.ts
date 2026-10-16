import { pathToFileURL } from 'node:url'

import type { MdxContextAdmission } from './mdx-context-admission'
import type { MdxLoaderOccurrence } from './mdx-context-lifetime'
import { admitMdxNativeCell } from './mdx-native-scope'
import { isMdxRecord } from './mdx-pipeline'
import { prepareMdxPlugins } from './mdx-prepare-plugins'
import {
  type MdxPrewarmStep,
  recognizeMdxPrewarmStep,
} from './mdx-prewarm-boundary'

type Callback = (error?: unknown, source?: unknown, map?: unknown) => void
type InstalledLoader = (
  this: object,
  source: string,
  callback: Callback,
) => void
function isInstalledLoader(value: unknown): value is InstalledLoader {
  return typeof value === 'function'
}

// Preparation only. Native bundler rules never reference this entry.
export default function mdxPrewarmLoader(
  context: object,
  control: {
    readonly step: MdxPrewarmStep | undefined
    readonly admission: MdxContextAdmission
    readonly occurrence: MdxLoaderOccurrence
  },
  source: unknown,
) {
  const { step, admission, occurrence } = control
  const raw = admission.view(context, occurrence)
  const asyncMethod: unknown = Reflect.get(raw, 'async')
  if (typeof asyncMethod !== 'function') throw new TypeError('missing async')
  const callback: Callback = Reflect.apply(asyncMethod, raw, [])
  const run = async () => {
    if (!step) throw new TypeError('MDX preparation bridge step is unavailable')
    const getOptions: unknown = Reflect.get(raw, 'getOptions')
    if (typeof getOptions !== 'function')
      throw new TypeError('missing getOptions')
    let options: unknown = Reflect.apply(getOptions, raw, [])
    if (step.wrapper) {
      if (!isMdxRecord(options))
        throw new TypeError('Next MDX options must be an object')
      const prepared = await prepareMdxPlugins(
        options,
        String(Reflect.get(raw, 'context')),
        step.wrapper,
      )
      options = prepared.options
      const addDependency: unknown = Reflect.get(raw, 'addDependency')
      if (typeof addDependency !== 'function')
        throw new TypeError('missing dependency API')
      for (const path of prepared.dependencies)
        Reflect.apply(addDependency, context, [path])
    }
    const mod: unknown = await import(pathToFileURL(step.module).href)
    if (!isMdxRecord(mod) || !isInstalledLoader(mod.loader))
      throw new TypeError('installed MDX ESM loader is unavailable')
    const scope = admitMdxNativeCell(mod, step.compiler, {
      options,
      sourceMap: Reflect.get(raw, 'sourceMap'),
      mode: Reflect.get(raw, 'mode'),
    })
    admission.admit(
      () =>
        recognizeMdxPrewarmStep(occurrence.loader)?.module === step.module &&
        scope.current(),
    )
    const proxy = admission.view(context, occurrence, {
      compiler: scope.compiler,
      options: () => options,
    })
    if (typeof source !== 'string')
      throw new TypeError('MDX bridge needs string source')
    mod.loader.call(proxy, source, callback)
  }
  void run().catch(callback)
}
