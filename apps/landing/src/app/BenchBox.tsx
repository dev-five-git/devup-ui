'use client'

import { Box } from '@devup-ui/react'
import { useRef } from 'react'
import { useCallback, useState } from 'react'
import { ReactNode } from 'react'

interface BenchBoxProps {
  children: ReactNode
}
export function BenchBox({ children }: BenchBoxProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const [isDragging, setIsDragging] = useState(false)
  const [startX, setStartX] = useState(0)
  const [scrollLeft, setScrollLeft] = useState(0)

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (!scrollRef.current) return

    setIsDragging(true)
    setStartX(e.pageX - scrollRef.current.offsetLeft)
    setScrollLeft(scrollRef.current.scrollLeft)
    scrollRef.current.style.cursor = 'grabbing'
    scrollRef.current.style.userSelect = 'none'
  }, [])

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!isDragging || !scrollRef.current) return

      e.preventDefault()
      const x = e.pageX - scrollRef.current.offsetLeft
      const walk = (x - startX) * 2 // 스크롤 속도 조절
      scrollRef.current.scrollLeft = scrollLeft - walk
    },
    [isDragging, startX, scrollLeft],
  )

  const handleMouseUp = useCallback(() => {
    if (!scrollRef.current) return

    setIsDragging(false)
    scrollRef.current.style.cursor = 'grab'
    scrollRef.current.style.userSelect = 'auto'
  }, [])

  const handleMouseLeave = useCallback(() => {
    if (!scrollRef.current) return

    setIsDragging(false)
    scrollRef.current.style.cursor = 'grab'
    scrollRef.current.style.userSelect = 'auto'
  }, [])

  return (
    <Box
      ref={scrollRef}
      WebkitOverflowScrolling="touch"
      cursor={['grab', null, null, null, 'default']}
      onMouseDown={handleMouseDown}
      onMouseLeave={handleMouseLeave}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      overflow={['auto', null, null, null, 'visible']}
      scrollbarWidth="none"
    >
      {children}
    </Box>
  )
}
