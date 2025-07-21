'use client'

import { css, useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconSelectComp({
  className,
  ...props
}: SVGProps<SVGSVGElement>) {
  const theme = useTheme()
  return (
    <svg
      className={className}
      fill="none"
      height="140"
      viewBox="0 0 259 140"
      width="259"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <rect
        height="71"
        rx="7.5"
        stroke={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        width="173"
        x="42.5"
        y="34.5"
      />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3A3A3A'}
        height="30"
        rx="5"
        width="166"
        x="46"
        y="38"
      />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="100"
        x="53"
        y="49"
      />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="86"
        x="53"
        y="82"
      />
    </svg>
  )
}
