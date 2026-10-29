'use client'

import Link from 'next/link'
import type { ComponentProps } from 'react'

import { localizeHref } from '@/i18n/config'
import { useLocale } from '@/i18n/LocaleProvider'

/** Preserve the selected language on internal site links. */
export function LocalizedLink({ href, ...props }: ComponentProps<typeof Link>) {
  const locale = useLocale()
  return (
    <Link
      {...props}
      href={typeof href === 'string' ? localizeHref(href, locale) : href}
    />
  )
}
