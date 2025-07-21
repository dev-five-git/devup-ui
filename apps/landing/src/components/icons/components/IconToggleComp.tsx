'use client'

import { css } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconToggleComp({
  className,
  ...props
}: SVGProps<SVGSVGElement>) {
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
        className={css({ fill: '$primary' })}
        height="28"
        rx="14"
        width="50"
        x="105"
        y="56"
      />
      <circle cx="141" cy="70" fill="white" r="10" />
    </svg>
  )
}
