import { existsSync } from 'node:fs'
import { dirname, resolve } from 'node:path'

import { parseImports } from '@devup-ui/plugin-utils'
import type { DefaultTreeAdapterTypes } from 'parse5'

export type HtmlReservationEntry = {
  readonly request: string
  readonly context: string
}

export type HtmlReservationOptions = {
  readonly command: 'build' | 'serve'
  readonly base?: string
  readonly root: string
  readonly filename: string
  readonly publicDir: string | false
  readonly env?: Readonly<Record<string, string | number | boolean | undefined>>
}

/** Physical discovery only: native HTML compilation and diagnostics stay native. */
export async function htmlReservationEntries(
  html: string,
  options: HtmlReservationOptions,
): Promise<readonly HtmlReservationEntry[]> {
  // Keep this public import asynchronous in BOTH published module formats.
  const { parse } = await import('parse5')
  const source = html.replace(/%(\S+?)%/g, (text: string, key: string) => {
    const value = options.env?.[key]
    return value === undefined ? text : String(value)
  })
  const entries: HtmlReservationEntry[] = []
  const context = dirname(options.filename)
  function add(request: string, scriptSource: boolean): void {
    if (/^(?:[a-z][\w+.-]*:|\/\/|#)/i.test(request)) return
    let path = request.split(/[?#]/)[0]
    if (!path) return
    // Dev HTTP middleware decodes URLs and removes the served base; build resolution does neither.
    if (options.command === 'serve') {
      try {
        path = decodeURI(path)
      } catch (error) {
        if (!(error instanceof URIError)) throw error
      }
      const base = options.base
      if (
        typeof base === 'string' &&
        base.startsWith('/') &&
        path.startsWith(base)
      )
        path = `/${path.slice(base.length)}`
    }
    if (
      path.startsWith('/') &&
      options.publicDir &&
      existsSync(resolve(options.publicDir, `.${path}`))
    )
      return
    entries.push({
      request: path.startsWith('/')
        ? resolve(options.root, `.${path}`)
        : scriptSource
          ? resolve(context, path)
          : path,
      context,
    })
  }
  function visit(node: DefaultTreeAdapterTypes.Node): void {
    if ('tagName' in node && node.tagName === 'template' && 'content' in node) {
      visit(node.content)
      return
    }
    if ('tagName' in node && node.tagName === 'script') {
      const attributes = new Map(
        node.attrs
          .filter((attribute) => attribute.prefix === undefined)
          .map(({ name, value }) => [name, value]),
      )
      if (
        attributes.get('type') === 'module' &&
        !attributes.has('vite-ignore')
      ) {
        const src = attributes.get('src')
        if (src) add(src, true)
        // Even an excluded src can have native-recognized inline contents.
        if (
          !src ||
          /^(?:[a-z][\w+.-]*:|\/\/|#)/i.test(src) ||
          (src.startsWith('/') &&
            options.publicDir &&
            existsSync(resolve(options.publicDir, `.${src.split(/[?#]/)[0]}`)))
        ) {
          const text = node.childNodes
            .flatMap((child) => ('value' in child ? [child.value] : []))
            .join('')
          for (const { specifier } of parseImports(
            `${options.filename}.js`,
            text,
          ))
            add(specifier, false)
        }
      }
    }
    if ('childNodes' in node) for (const child of node.childNodes) visit(child)
  }
  visit(parse(source, { scriptingEnabled: false }))
  return entries
}
