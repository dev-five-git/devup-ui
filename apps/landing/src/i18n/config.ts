export const locales = ['en', 'ko'] as const
export type Locale = (typeof locales)[number]
export const defaultLocale: Locale = 'en'

export function stripLocale(path: string): string {
  return (
    path.replace(/^\/ko(?=\/|$|[?#])/u, '').replace(/^(?=[?#])/u, '/') || '/'
  )
}

export function getLocale(path: string): Locale {
  return /^\/ko(?:\/|$|[?#])/u.test(path) ? 'ko' : defaultLocale
}

/** Keep fragments, queries, assets and external links intact. */
export function localizeHref(href: string, locale: Locale): string {
  if (!href.startsWith('/') || href.startsWith('//')) return href
  const path = stripLocale(href)
  if (
    !/^\/(?:docs(?:\/|$)|components(?:\/|$)|showcase(?:[?#]|$)|team(?:[?#]|$)|[?#]|$)/u.test(
      path,
    )
  ) {
    return href
  }
  return locale === 'en' ? path : `/ko${path === '/' ? '' : path}`
}
