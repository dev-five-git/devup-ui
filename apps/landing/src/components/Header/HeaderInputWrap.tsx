'use client'
import { usePathname } from 'next/navigation'

interface HeaderInputWrapProps {
  children: React.ReactNode
}

export function HeaderInputWrap({ children }: HeaderInputWrapProps) {
  const path = usePathname()
  const root = path === '/'
  return root ? null : children
}
