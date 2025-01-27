'use client'
import { useSearchParams } from 'next/navigation'

interface MobMenuWrapperProps {
  openChildren?: React.ReactNode
  children: React.ReactNode
}

export function MobMenuWrapper({
  children,
  openChildren,
}: MobMenuWrapperProps) {
  const menu = useSearchParams().get('menu') === '1'
  return menu ? openChildren : children
}
