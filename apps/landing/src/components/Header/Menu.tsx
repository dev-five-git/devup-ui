'use client'
import { Text } from '@devup-ui/react'
import { usePathname } from 'next/navigation'

import { stripLocale } from '@/i18n/config'
import { Message } from '@/i18n/Message'

interface MenuProps {
  children?: React.ReactNode
  keyword: string
}

export function Menu({ children, keyword }: MenuProps) {
  const path = stripLocale(usePathname())
  const selected = path.startsWith(`/${keyword}`)
  return (
    <Text
      _active={{
        opacity: '1',
        color: '$primary',
      }}
      _hover={{
        opacity: '1',
      }}
      color={selected ? '$primary' : '$title'}
      opacity={selected ? undefined : '0.6'}
      typography="buttonLsemiB"
    >
      {typeof children === 'string' ? <Message>{children}</Message> : children}
    </Text>
  )
}
