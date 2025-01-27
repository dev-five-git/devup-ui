'use client'
import { usePathname } from 'next/navigation'

import { isRoot } from '../../utils/is-root'

interface HeaderInputWrapProps {
  children: React.ReactNode
}

export function HeaderInputWrap({ children }: HeaderInputWrapProps) {
  const path = usePathname()
  const root = isRoot(path)
  return root ? null : children
}
