'use client'

import { css } from '@devup-ui/react'
import { usePathname } from 'next/navigation'

import { localizeHref } from '@/i18n/config'
import { useLocale } from '@/i18n/LocaleProvider'

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
      href={localizeHref(pathname, targetLocale)}
      hrefLang={targetLocale}
      lang={targetLocale}
      onClick={(event) => {
        event.currentTarget.href = `${localizeHref(pathname, targetLocale)}${window.location.search}${window.location.hash}`
      }}
    >
      {targetLocale === 'ko' ? '한국어' : 'English'}
    </a>
  )
}
