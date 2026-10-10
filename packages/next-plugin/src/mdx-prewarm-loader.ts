import { pathToFileURL } from 'node:url'

import { isMdxRecord } from './mdx-pipeline'
import { prepareMdxPlugins } from './mdx-prepare-plugins'
import type { MdxPrewarmStep } from './mdx-prewarm-boundary'

type Callback = (error?: unknown, source?: unknown, map?: unknown) => void
type Context = {
  readonly loaderIndex: number
  readonly context: string
  readonly devupMdxPrewarm: ReadonlyMap<number, MdxPrewarmStep>
  readonly getOptions: () => unknown
  readonly async: () => Callback
  readonly addDependency: (path: string) => void
}
type InstalledLoader = (
  this: Context,
  source: string,
  callback: Callback,
) => void
function isInstalledLoader(value: unknown): value is InstalledLoader {
  return typeof value === 'function'
}

// Preparation only. Native bundler rules never reference this entry.
export default function mdxPrewarmLoader(this: Context, source: string) {
  const callback = this.async()
  const run = async () => {
    const step = this.devupMdxPrewarm.get(this.loaderIndex)
    if (!step) throw new TypeError('MDX preparation bridge step is unavailable')
    let options = this.getOptions()
    if (step.wrapper) {
      if (!isMdxRecord(options))
        throw new TypeError('Next MDX options must be an object')
      const prepared = await prepareMdxPlugins(
        options,
        this.context,
        step.wrapper,
      )
      options = prepared.options
      for (const path of prepared.dependencies) this.addDependency(path)
    }
    const mod: unknown = await import(pathToFileURL(step.module).href)
    if (!isMdxRecord(mod) || !isInstalledLoader(mod.loader))
      throw new TypeError('installed MDX ESM loader is unavailable')
    const proxy = new Proxy(this, {
      get(target, property, receiver) {
        if (property === 'getOptions') return () => options
        if (property === '_compiler') return step.compiler
        return Reflect.get(target, property, receiver)
      },
    })
    mod.loader.call(proxy, source, callback)
  }
  void run().catch(callback)
}
