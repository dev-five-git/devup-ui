import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { ThemeScript } from '../ThemeScript'

const KEY = '__DF_THEME_SELECTED__'
const originalMatchMedia = window.matchMedia
const originalStorage = Object.getOwnPropertyDescriptor(window, 'localStorage')!

/** A storage that throws on reads, as a browser that blocks storage does */
function blockStorage(message: string) {
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem: () => {
        throw new DOMException(message, 'SecurityError')
      },
    },
  })
}
const originalAdd = window.addEventListener
const listening: [string, EventListenerOrEventListenerObject][] = []

function run() {
  const { container } = render(<ThemeScript />)
  // The inline script as the browser runs it
  new Function(container.querySelector('script')!.textContent!)()
}

function fakeSystem(dark: boolean) {
  const listeners = new Set<() => void>()
  const query = {
    matches: dark,
    addEventListener: (_: string, listener: () => void) =>
      listeners.add(listener),
  }
  window.matchMedia = (() => query) as unknown as typeof window.matchMedia
  return (next: boolean) => {
    query.matches = next
    listeners.forEach((listener) => listener())
  }
}

beforeEach(() => {
  // The page ends with the script's listeners: remove them so no test hears another's events
  window.addEventListener = ((type, listener, options) => {
    listening.push([type, listener])
    originalAdd.call(window, type, listener, options)
  }) as typeof window.addEventListener
  localStorage.clear()
  document.documentElement.removeAttribute('data-theme')
})

afterEach(() => {
  listening
    .splice(0)
    .forEach(([type, listener]) => window.removeEventListener(type, listener))
  window.matchMedia = originalMatchMedia
  Object.defineProperty(window, 'localStorage', originalStorage)
  window.addEventListener = originalAdd
  localStorage.clear()
})

describe('ThemeScript under a content security policy', () => {
  it('passes a nonce and the other script attributes to the script element', () => {
    const { container } = render(
      <ThemeScript data-purpose="theme" id="theme" nonce="n0nce" />,
    )
    const script = container.querySelector('script')!

    expect(
      script.getAttribute('nonce') ?? (script as HTMLScriptElement).nonce,
    ).toBe('n0nce')
    expect(script).toHaveAttribute('id', 'theme')
    expect(script).toHaveAttribute('data-purpose', 'theme')
  })

  it('writes its own script only', () => {
    const { container } = render(<ThemeScript nonce="n" />)

    expect(container.querySelectorAll('script')).toHaveLength(1)
    expect(container.querySelector('script')?.textContent).toContain(
      'data-theme',
    )
  })
})

describe('ThemeScript sets a theme whatever the browser allows', () => {
  it('sets the saved theme', () => {
    localStorage.setItem(KEY, 'light')
    fakeSystem(true)
    run()

    expect(document.documentElement.dataset.theme).toBe('light')
  })

  it('sets the dark theme for a system that prefers it, and the default for one that does not', () => {
    const change = fakeSystem(true)
    run()
    expect(document.documentElement.dataset.theme).toBe('dark')

    change(false)
    expect(document.documentElement.dataset.theme).toBe('default')
  })

  it('does not abort when the storage throws', () => {
    blockStorage('blocked')
    fakeSystem(true)
    run()

    expect(document.documentElement.dataset.theme).toBe('dark')
  })

  it('does not abort when there is no matchMedia, and sets the default theme', () => {
    // @ts-expect-error an environment without media queries
    window.matchMedia = undefined
    run()

    expect(document.documentElement.dataset.theme).toBe('default')
  })

  it('does not abort when the media query throws', () => {
    window.matchMedia = (() => {
      throw new Error('no media queries')
    }) as unknown as typeof window.matchMedia
    run()

    expect(document.documentElement.dataset.theme).toBe('default')
  })

  it('does not abort when listening throws', () => {
    window.addEventListener = (() => {
      throw new Error('no listeners')
    }) as unknown as typeof window.addEventListener
    fakeSystem(false)
    run()

    expect(document.documentElement.dataset.theme).toBe('default')
  })

  it('does not follow the system when it is not auto', () => {
    const change = fakeSystem(true)
    const { container } = render(<ThemeScript auto={false} />)
    new Function(container.querySelector('script')!.textContent!)()

    expect(document.documentElement.dataset.theme).toBe('default')
    change(false)
    expect(document.documentElement.dataset.theme).toBe('default')
  })

  it('follows the choice another tab saves', () => {
    fakeSystem(false)
    run()
    expect(document.documentElement.dataset.theme).toBe('default')

    localStorage.setItem(KEY, 'dark')
    window.dispatchEvent(new StorageEvent('storage', { key: KEY }))
    expect(document.documentElement.dataset.theme).toBe('dark')

    localStorage.setItem(KEY, 'light')
    window.dispatchEvent(new StorageEvent('storage', { key: 'other' }))
    expect(document.documentElement.dataset.theme).toBe('dark')

    window.dispatchEvent(new StorageEvent('storage', { key: null }))
    expect(document.documentElement.dataset.theme).toBe('light')
  })

  it('sets a theme it is given without reading anything', () => {
    blockStorage('must not read')
    const { container } = render(<ThemeScript theme={'brand' as never} />)
    new Function(container.querySelector('script')!.textContent!)()

    expect(document.documentElement.dataset.theme).toBe('brand')
  })
})
