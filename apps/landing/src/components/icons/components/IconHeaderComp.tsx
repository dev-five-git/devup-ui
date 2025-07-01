'use client'

import { css, useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconHeaderComp({
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
      <path
        d="M43 102H217V110C217 114.418 213.418 118 209 118H51C46.5817 118 43 114.418 43 110V102Z"
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
      />
      <rect
        height="94"
        rx="7.5"
        stroke={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        width="173"
        x="43.5"
        y="23.5"
      />
      <path
        d="M43 31C43 26.5817 46.5817 23 51 23H209C213.418 23 217 26.5817 217 31V36H43V31Z"
        fill="#363636"
      />
      <circle cx="52" cy="30" fill="#E13953" r="3" />
      <circle cx="61" cy="30" fill="#F4DF44" r="3" />
      <circle cx="70" cy="30" fill="#4FBD56" r="3" />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
        height="8"
        rx="2"
        width="60"
        x="61"
        y="54"
      />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
        height="28"
        rx="2"
        width="138"
        x="61"
        y="68"
      />
      <rect
        className={css({ fill: '$primary' })}
        height="10"
        width="174"
        x="43"
        y="36"
      />
    </svg>
  )
}
