import { stripLocale } from '../i18n/config'

export function isRoot(path: string) {
  return '/' === stripLocale(path)
}
