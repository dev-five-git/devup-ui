'use client'

import { useTranslations } from 'next-intl'

/** PropsTable is a server component rendered from MDX, so it reads its labels here. */
export function PropsTableLabel({
  name,
}: {
  name: 'default' | 'description' | 'empty' | 'prop' | 'type'
}) {
  const t = useTranslations('PropsTable')
  return t(name)
}
