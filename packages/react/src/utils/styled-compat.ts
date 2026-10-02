import type { ReactElement, ReactNode } from 'react'

/**
 * styled-components SSR plumbing, kept as no-ops.
 *
 * Those APIs exist to collect runtime-injected styles and flush them into the
 * server response. Devup UI writes every rule into a real stylesheet at build
 * time, so there is nothing to collect — the shims let existing call sites keep
 * compiling while producing no markup of their own.
 */
export class ServerStyleSheet {
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

export function isStyledComponent(_target: unknown): boolean {
  return false
}
