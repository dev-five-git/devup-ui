'use client'

import { DevupTheme } from '@devup-ui/react'

type Theme = keyof DevupTheme | null
type StoreChangeEvent = (newTheme: Theme) => void

const initTheme = null

export const themeStore = (() => {
  if (typeof window === 'undefined')
    return {
      get: () => initTheme,
      set: () => {},
      subscribe: () => () => {},
    }
  const el = document.documentElement

  const subscribers: Set<StoreChangeEvent> = new Set()
  let theme: Theme = initTheme
  const get = () => {
    return theme
  }
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
    mutations.forEach((mutation) => {
      if (
        mutation.type === 'attributes' &&
        mutation.target instanceof HTMLElement
      ) {
        set(mutation.target.getAttribute('data-theme') as Theme)
      }
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
})()
