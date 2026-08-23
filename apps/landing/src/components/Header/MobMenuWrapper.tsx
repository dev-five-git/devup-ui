'use client'

import { useQueryParam } from '@/utils/use-query-param'

interface MobMenuWrapperProps {
  openChildren?: React.ReactNode
  children: React.ReactNode
}

export function MobMenuWrapper({
  children,
  openChildren,
}: MobMenuWrapperProps) {
  const menu = useQueryParam('menu') === '1'
  return menu ? openChildren : children
}
