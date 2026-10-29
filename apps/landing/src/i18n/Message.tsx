'use client'

import { useLocale } from './LocaleProvider'
import { translateMessage } from './messages'

/** Translate UI copy while retaining the English source as a fallback. */
export function Message({ children }: { children: string }) {
  return translateMessage(children, useLocale())
}
