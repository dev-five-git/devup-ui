'use client'

import { createContext, use } from 'react'

import { defaultLocale, type Locale } from './config'

const LocaleContext = createContext<Locale>(defaultLocale)

/** Share the URL locale with navigation without sending page content to the client. */
export function LocaleProvider({
  locale,
  children,
}: {
  locale: Locale
  children: React.ReactNode
}) {
  return <LocaleContext value={locale}>{children}</LocaleContext>
}

export function useLocale() {
  return use(LocaleContext)
}
