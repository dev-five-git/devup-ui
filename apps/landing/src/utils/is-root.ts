import { URL_PREFIX } from '../constants'

export function isRoot(path: string) {
  return URL_PREFIX + '/' === path || '/' === path
}
