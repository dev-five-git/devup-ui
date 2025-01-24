'use client'
import { Text } from '@devup-ui/react'
import { usePathname } from 'next/navigation'

interface MenuProps {
  children?: React.ReactNode
  keyword: string
}

export function Menu({ children, keyword }: MenuProps) {
  const path = usePathname()
  const selected = path.startsWith(`/${keyword}`)
  return (
    <Text
      color={selected ? '$primary' : '$title'}
      opacity={selected ? 1 : '0.6'}
      typography="buttonLsemiB"
    >
      {children}
    </Text>
  )
}
