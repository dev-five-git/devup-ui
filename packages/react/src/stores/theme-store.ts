'use client'
import type { DevupTheme } from '../types/theme'

type Theme = keyof DevupTheme | null
type StoreChangeEvent = (newTheme: Theme) => void

function createClientThemeStore() {
  const el = document.documentElement
  const subscribers: Set<StoreChangeEvent> = new Set()
  let theme: Theme = null
  const get = () => theme
  const set = (newTheme: Theme) => {
    theme = newTheme
    subscribers.forEach((subscriber) => subscriber(theme))
  }

  const subscribe = (onStoreChange: StoreChangeEvent) => {
    subscribers.add(onStoreChange)
    set(el.getAttribute('data-theme') as Theme)
    return () => subscribers.delete(onStoreChange)
  }

  const mo = new MutationObserver((mutations) => {
    mutations
      .filter((m) => m.type === 'attributes' && m.target instanceof HTMLElement)
      .forEach((m) => {
        set((m.target as HTMLElement).getAttribute('data-theme') as Theme)
      })
  })
  mo.observe(el, {
    attributes: true,
    attributeFilter: ['data-theme'],
    subtree: false,
  })
  return {
    get,
    set,
    subscribe,
  }
}

const serverThemeStore: ReturnType<typeof createClientThemeStore> = {
  get: () => null,
  set: () => {},
  subscribe: () => () => {},
} as unknown as ReturnType<typeof createClientThemeStore>

export function createServerThemeStore() {
  return serverThemeStore
}

export const createThemeStore: typeof createClientThemeStore =
  typeof window === 'undefined'
    ? createServerThemeStore
    : createClientThemeStore
