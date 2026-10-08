declare global {
  var __devupUiServedCss: WeakMap<object, Map<string, string>> | undefined
}

/**
 * The stylesheet each devup-ui CSS module of `compilation` was built with, by
 * resource path. The plugin and its loaders load as separate modules, so the
 * record lives on `globalThis`.
 */
export function servedCss(compilation: object): Map<string, string> {
  globalThis.__devupUiServedCss ??= new WeakMap()
  let served = globalThis.__devupUiServedCss.get(compilation)
  if (!served) {
    served = new Map()
    globalThis.__devupUiServedCss.set(compilation, served)
  }
  return served
}
