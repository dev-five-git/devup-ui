'use client'

import { css, useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconConfirmComp({
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
        height="94"
        rx="7.5"
        stroke={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        width="173"
        x="42.5"
        y="22.5"
      />
      <circle cx="66" cy="46" fill="#F3B238" r="8.33333" />
      <path
        d="M66.0003 48.5C66.6905 48.5002 67.2503 49.0598 67.2503 49.75C67.2503 50.4402 66.6905 50.9998 66.0003 51C65.31 51 64.7503 50.4404 64.7503 49.75C64.7503 49.0596 65.31 48.5 66.0003 48.5ZM66.4164 41C66.8766 41 67.6664 41.3321 67.6664 41.7412L67.2503 46.5557C67.2503 47.1692 66.6905 47.6668 66.0003 47.667C65.31 47.667 64.7504 47.1693 64.7503 46.5557L64.3333 41.7412C64.3333 41.3321 65.1231 41 65.5833 41H66.4164Z"
        fill="white"
      />
      <rect
        className={css({ fill: '$text' })}
        height="6"
        opacity="0.7"
        rx="2"
        width="40"
        x="57"
        y="65"
      />
      <rect
        className={css({ fill: '$text' })}
        height="6"
        opacity="0.4"
        rx="2"
        width="86"
        x="57"
        y="76"
      />
      <rect
        height="15"
        rx="3.5"
        stroke={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        width="41"
        x="121.5"
        y="93.5"
      />
      <rect
        className={css({ fill: '$primary' })}
        height="16"
        rx="4"
        width="42"
        x="166"
        y="93"
      />
    </svg>
  )
}
