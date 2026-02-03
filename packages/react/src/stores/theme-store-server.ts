import type { createThemeStore } from './theme-store'

const serverThemeStore = {
  get: () => null,
  set: () => {},
  subscribe: () => () => {},
}

export function createServerThemeStore(): ReturnType<typeof createThemeStore> {
  return serverThemeStore as unknown as ReturnType<typeof createThemeStore>
}
