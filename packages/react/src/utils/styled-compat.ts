import type { ReactElement, ReactNode } from 'react'

/**
 * What `ServerStyleSheet.instance` gives: the sheet SSR setups pass to
 * `StyleSheetManager sheet={...}` and clear after flushing (as the Next.js
 * App Router registry calls `instance.clearTag()`). It holds no rules.
 */
export class StyleSheetInstance {
  clearTag(): void {}

  toString(): string {
    return ''
  }
}

/**
 * styled-components SSR plumbing, kept as no-ops.
 *
 * Those APIs exist to collect runtime-injected styles and flush them into the
 * server response. Devup UI writes every rule into a real stylesheet at build
 * time, so there is nothing to collect — the shims let existing call sites keep
 * compiling while producing no markup of their own.
 */
export class ServerStyleSheet {
  readonly instance = new StyleSheetInstance()

  constructor() {}

  collectStyles(children: ReactNode): ReactNode {
    return children
  }

  getStyleTags(): string {
    return ''
  }

  getStyleElement(): ReactElement[] {
    return []
  }

  seal(): void {}
}

export function StyleSheetManager({
  children,
}: {
  children?: ReactNode
  [key: string]: unknown
}): ReactNode {
  return children
}

/**
 * Emotion's `CacheProvider`, kept as a no-op: the cache it configures holds
 * styles Emotion would insert at runtime, and Devup UI writes them at build time
 */
export function CacheProvider({
  children,
}: {
  value?: unknown
  children?: ReactNode
}): ReactNode {
  return children
}

/**
 * styled-components' `isStyledComponent`: the build marks every component it
 * compiles from `styled` with a `styledComponentId` property, the one
 * styled-components itself checks for
 */
export function isStyledComponent(target: unknown): boolean {
  return (
    (typeof target === 'object' || typeof target === 'function') &&
    target !== null &&
    'styledComponentId' in target
  )
}
