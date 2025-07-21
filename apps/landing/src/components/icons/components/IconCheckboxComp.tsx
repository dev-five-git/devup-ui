'use client'

import { css, useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconCheckboxComp({
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
        rx="1.5"
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
      <rect fill="white" height="18" rx="2" width="18" x="67" y="61" />
      <rect
        className={css({ fill: '$primary' })}
        height="18"
        rx="2"
        width="18"
        x="67"
        y="61"
      />
      <path
        clipRule="evenodd"
        d="M81.6474 66.8071C82.0992 67.2337 82.1195 67.9457 81.6929 68.3975L75.3179 75.1475C75.1021 75.3759 74.8007 75.5037 74.4865 75.4999C74.1723 75.4961 73.874 75.3611 73.6638 75.1276L70.2888 71.3776C69.8732 70.9158 69.9106 70.2044 70.3724 69.7888C70.8342 69.3732 71.5456 69.4106 71.9612 69.8724L74.5199 72.7154L80.0571 66.8526C80.4837 66.4008 81.1957 66.3805 81.6474 66.8071Z"
        fill="white"
        fillRule="evenodd"
      />
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
        rx="1.5"
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
