import { afterEach, beforeEach, describe, expect, it, mock } from 'bun:test'

import { getTheme } from '../get-theme'
import { initTheme as init } from '../init-theme'
import { setTheme } from '../set-theme'

const KEY = '__DF_THEME_SELECTED__'
const root = () => document.documentElement

interface FakeQuery {
  matches: boolean
  listeners: Set<() => void>
  addEventListener: (type: string, listener: () => void) => void
  removeEventListener: (type: string, listener: () => void) => void
}

function fakeSystem(dark: boolean) {
  const query: FakeQuery = {
    matches: dark,
    listeners: new Set(),
    addEventListener: (_type, listener) => query.listeners.add(listener),
    removeEventListener: (_type, listener) => query.listeners.delete(listener),
  }
  window.matchMedia = ((q: string) => {
    expect(q).toBe('(prefers-color-scheme:dark)')
    return query
  }) as unknown as typeof window.matchMedia
  return {
    query,
    change(next: boolean) {
      query.matches = next
      query.listeners.forEach((listener) => listener())
    },
  }
}

/** A storage whose method throws as a browser that blocks storage does; gives back the storage it replaced */
function blockStorage(method: 'getItem' | 'setItem') {
  const original = Object.getOwnPropertyDescriptor(window, 'localStorage')!
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem: () => null,
      setItem: () => {},
      [method]: () => {
        throw new DOMException('blocked', 'SecurityError')
      },
    },
  })
  return () => Object.defineProperty(window, 'localStorage', original)
}

function storageEvent(key: string | null) {
  window.dispatchEvent(new StorageEvent('storage', { key }))
}

const originalMatchMedia = window.matchMedia
const running: (() => void)[] = []

/** The theme initialization, stopped when the test ends so no test hears another's events */
function initTheme(...args: Parameters<typeof init>) {
  const stop = init(...args)
  running.push(stop)
  return stop
}

beforeEach(() => {
  localStorage.clear()
  root().removeAttribute('data-theme')
})

afterEach(() => {
  running.splice(0).forEach((stop) => stop())
  window.matchMedia = originalMatchMedia
  localStorage.clear()
})

describe('theme helpers on the server', () => {
  it('read and write nothing where there is no page', () => {
    const { document: page } = globalThis
    // @ts-expect-error the server has no document
    globalThis.document = undefined
    try {
      expect(getTheme()).toBeNull()
      expect(() => setTheme('dark' as never)).not.toThrow()
      const stop = initTheme(true)
      expect(() => stop()).not.toThrow()
      expect(() => initTheme(false, 'dark' as never)()).not.toThrow()
    } finally {
      globalThis.document = page
    }
    expect(localStorage.getItem(KEY)).toBeNull()
  })
})

describe('setTheme', () => {
  it('sets the theme and saves it as an explicit choice', () => {
    setTheme('dark' as never)

    expect(getTheme()).toBe('dark' as never)
    expect(localStorage.getItem(KEY)).toBe('dark')
  })

  it('still sets the theme when the storage cannot be written', () => {
    const restore = blockStorage('setItem')
    try {
      expect(() => setTheme('dark' as never)).not.toThrow()
    } finally {
      restore()
    }

    expect(getTheme()).toBe('dark' as never)
  })
})

describe('initTheme follows the system and other tabs', () => {
  it('uses the system preference while no theme was chosen, and follows its changes', () => {
    const system = fakeSystem(true)
    const stop = initTheme(true)
    expect(getTheme()).toBe('dark' as never)

    system.change(false)
    expect(getTheme()).toBe('default' as never)
    system.change(true)
    expect(getTheme()).toBe('dark' as never)

    stop()
    system.change(false)
    expect(getTheme()).toBe('dark' as never)
    expect(system.query.listeners.size).toBe(0)
  })

  it('keeps an explicit choice against the system', () => {
    const system = fakeSystem(true)
    localStorage.setItem(KEY, 'light')
    initTheme(true)

    expect(getTheme()).toBe('light' as never)
    system.change(false)
    system.change(true)
    expect(getTheme()).toBe('light' as never)
  })

  it('ignores the system unless it was asked to follow it', () => {
    const system = fakeSystem(true)
    const stop = initTheme(false)

    expect(getTheme()).toBe('default' as never)
    expect(system.query.listeners.size).toBe(0)
    stop()
  })

  it('follows the theme another tab saves, and goes back when it is cleared', () => {
    const system = fakeSystem(true)
    const stop = initTheme(true)
    expect(getTheme()).toBe('dark' as never)

    localStorage.setItem(KEY, 'light')
    storageEvent(KEY)
    expect(getTheme()).toBe('light' as never)

    localStorage.removeItem(KEY)
    storageEvent(KEY)
    expect(getTheme()).toBe('dark' as never)

    localStorage.setItem(KEY, 'light')
    storageEvent(null)
    expect(getTheme()).toBe('light' as never)

    localStorage.setItem(KEY, 'unrelated')
    storageEvent('another-key')
    expect(getTheme()).toBe('light' as never)

    stop()
    localStorage.setItem(KEY, 'dark')
    storageEvent(KEY)
    expect(getTheme()).toBe('light' as never)
    expect(system.query.listeners.size).toBe(0)
  })

  it('applies a theme it is given and follows nothing', () => {
    const system = fakeSystem(true)
    const stop = initTheme(true, 'brand' as never)

    expect(getTheme()).toBe('brand' as never)
    system.change(false)
    expect(getTheme()).toBe('brand' as never)
    stop()
  })

  it('falls back to the default theme when the storage and the media query throw', () => {
    const restore = blockStorage('getItem')
    window.matchMedia = (() => {
      throw new Error('no media queries')
    }) as unknown as typeof window.matchMedia
    try {
      expect(() => localStorage.getItem('x')).toThrow()
      const stop = initTheme(true)
      expect(getTheme()).toBe('default' as never)
      expect(() => stop()).not.toThrow()
    } finally {
      restore()
    }
  })

  it('does not throw where there is no matchMedia', () => {
    // @ts-expect-error an environment without media queries
    window.matchMedia = undefined
    const onThrow = mock()
    try {
      initTheme(true)()
    } catch (error) {
      onThrow(error)
    }

    expect(onThrow).not.toHaveBeenCalled()
    expect(getTheme()).toBe('default' as never)
  })
})
