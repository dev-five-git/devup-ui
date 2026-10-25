import type { build } from 'vite'

export function nativeWatcher(result: Awaited<ReturnType<typeof build>>) {
  if (Array.isArray(result) || !('on' in result))
    throw new TypeError('Expected native watcher')
  return result
}
