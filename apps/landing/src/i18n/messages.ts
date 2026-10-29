import type { Locale } from './config'
import korean from './messages/ko.json'

const messages = new Map<string, string>(Object.entries(korean))

/** English UI text is the key and fallback for incremental translations. */
export function translateMessage(message: string, locale: Locale): string {
  return locale === 'ko' ? (messages.get(message) ?? message) : message
}
