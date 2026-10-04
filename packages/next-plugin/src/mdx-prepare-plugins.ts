import { createRequire } from 'node:module'
import { pathToFileURL } from 'node:url'

// This is the verified @next/mdx 16.3.6 import protocol, not a resolver.
export async function prepareMdxPlugins(
  options: Readonly<Record<string, unknown>>,
  context: string,
  wrapper: string,
) {
  const wrapperRequire = createRequire(wrapper)
  const dependencies: string[] = []
  async function importPlugin(plugin: unknown): Promise<unknown> {
    const specifier = Array.isArray(plugin) ? plugin[0] : plugin
    if (typeof specifier !== 'string') return plugin
    const path = wrapperRequire.resolve(specifier, { paths: [context] })
    const mod: unknown = await import(
      process.platform === 'win32' ? pathToFileURL(path).href : path
    )
    const imported =
      typeof mod === 'object' && mod !== null && 'default' in mod
        ? mod.default || mod
        : mod
    dependencies.push(path)
    if (Array.isArray(plugin)) {
      plugin[0] = imported
      return plugin
    }
    return imported
  }
  const {
    recmaPlugins = [],
    rehypePlugins = [],
    remarkPlugins = [],
    ...rest
  } = options
  const updated = await Promise.all(
    [recmaPlugins, rehypePlugins, remarkPlugins].map((plugins) => {
      if (!Array.isArray(plugins))
        throw new TypeError('configured plugins must be an array')
      return Promise.all(plugins.map(importPlugin))
    }),
  )
  const [recma, rehype, remark] = updated
  return {
    options: {
      ...rest,
      recmaPlugins: recma,
      rehypePlugins: rehype,
      remarkPlugins: remark,
    },
    dependencies,
  }
}
