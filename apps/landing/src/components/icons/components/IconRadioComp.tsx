'use client'

import { css, useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconRadioComp({
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
        height="17"
        rx="8.5"
        stroke={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        width="17"
        x="67.5"
        y="33.5"
      />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="100"
        x="93"
        y="38"
      />
      <rect
        className={css({ stroke: '$primary' })}
        height="17"
        rx="8.5"
        width="17"
        x="67.5"
        y="61.5"
      />
      <circle className={css({ fill: '$primary' })} cx="76" cy="70" r="6" />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="100"
        x="93"
        y="66"
      />
      <rect
        height="17"
        rx="8.5"
        stroke={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        width="17"
        x="67.5"
        y="89.5"
      />
      <rect
        className={css({ fill: '$text' })}
        height="8"
        opacity="0.4"
        rx="2"
        width="100"
        x="93"
        y="94"
      />
    </svg>
  )
}
