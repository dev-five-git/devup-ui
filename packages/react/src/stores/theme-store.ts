'use client'
import type { DevupTheme } from '../types/theme'

type Theme = keyof DevupTheme | null
type StoreChangeEvent = (newTheme: Theme) => void

const initTheme = null

export function createThemeStore() {
  if (typeof window === 'undefined')
    return {
      get: () => initTheme,
      set: () => {},
      subscribe: () => () => {},
    }

  const el = document.documentElement
  const subscribers: Set<StoreChangeEvent> = new Set()
  let theme: Theme = initTheme
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
    for (const m of mutations)
      if (m.type === 'attributes' && m.target instanceof HTMLElement)
        set(m.target.getAttribute('data-theme') as Theme)
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
