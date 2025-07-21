'use client'

import { css, useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconFooterComp({
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
        y="23.5"
      />
      <path
        d="M42 31C42 26.5817 45.5817 23 50 23H208C212.418 23 216 26.5817 216 31V36H42V31Z"
        fill="#363636"
      />
      <circle cx="51" cy="30" fill="#E13953" r="3" />
      <circle cx="60" cy="30" fill="#F4DF44" r="3" />
      <circle cx="69" cy="30" fill="#4FBD56" r="3" />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
        height="8"
        rx="2"
        width="60"
        x="60"
        y="44"
      />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
        height="28"
        rx="2"
        width="138"
        x="60"
        y="58"
      />
      <path
        className={css({ fill: '$primary' })}
        d="M42 97H216V110C216 114.418 212.418 118 208 118H50C45.5817 118 42 114.418 42 110V97Z"
      />
    </svg>
  )
}
