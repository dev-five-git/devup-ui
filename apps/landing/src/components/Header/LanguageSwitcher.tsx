'use client'

import { css } from '@devup-ui/react'
import { useLocale } from 'next-intl'

import { getPathname, usePathname } from '@/i18n/navigation'

/** Use full navigation between root layouts so the document language is correct. */
export function LanguageSwitcher() {
  const locale = useLocale()
  const pathname = usePathname()
  const targetLocale = locale === 'en' ? 'ko' : 'en'

  return (
    <a
      aria-label={
        targetLocale === 'ko' ? 'Switch to Korean' : 'Switch to English'
      }
      className={css({
        color: '$text',
        typography: 'buttonS',
        textDecoration: 'none',
        whiteSpace: 'nowrap',
        px: 2,
        _hover: { color: '$primary' },
      })}
      href={getPathname({ href: pathname, locale: targetLocale })}
      hrefLang={targetLocale}
      lang={targetLocale}
      onClick={(event) => {
        event.currentTarget.href = `${getPathname({ href: pathname, locale: targetLocale })}${window.location.search}${window.location.hash}`
      }}
    >
      {targetLocale === 'ko' ? '한국어' : 'English'}
    </a>
  )
}
