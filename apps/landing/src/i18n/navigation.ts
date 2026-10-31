'use client'

import { createNavigation } from 'next-intl/navigation'

// Explicit routes keep English URLs unprefixed on the static host.
export const { Link, usePathname, getPathname } = createNavigation({
  locales: ['en', 'ko'],
  defaultLocale: 'en',
  localePrefix: 'as-needed',
  localeCookie: false,
})
