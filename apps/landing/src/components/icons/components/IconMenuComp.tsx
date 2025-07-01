'use client'

import { css, useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconMenuComp({
  className,
  ...props
}: SVGProps<SVGSVGElement>) {
  const theme = useTheme()
  return (
    <svg
      className={className}
      fill="none"
      height="140"
      viewBox="0 0 260 140"
      width="260"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <rect
        fill={theme === 'light' ? '#FBFBFB' : '#262626'}
        height="108"
        rx="4"
        width="140"
        x="60"
        y="16"
      />
      <rect
        fill={theme === 'light' ? '#EDECF7' : '#3f3f3f'}
        height="30"
        rx="5"
        width="130"
        x="65"
        y="53"
      />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="66"
        x="74"
        y="30"
      />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="66"
        x="74"
        y="64"
      />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="66"
        x="74"
        y="98"
      />
    </svg>
  )
}
