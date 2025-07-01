'use client'

import { css } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconSliderComp({
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
        className={css({ fill: '$border' })}
        height="6"
        rx="3"
        width="180"
        x="40"
        y="67"
      />
      <rect
        className={css({ fill: '$primary' })}
        height="6"
        width="120"
        x="40"
        y="67"
      />
      <circle
        className={css({ fill: '$primary' })}
        cx="160"
        cy="70.5"
        fillOpacity="0.2"
        r="16"
      />
      <g filter="url(#filter0_d_560_2659)">
        <circle cx="160" cy="70.5" fill="white" r="8" />
      </g>
      <defs>
        <filter
          colorInterpolationFilters="sRGB"
          filterUnits="userSpaceOnUse"
          height="24"
          id="filter0_d_560_2659"
          width="24"
          x="148"
          y="58.5"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feColorMatrix
            in="SourceAlpha"
            result="hardAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="2" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.2 0"
          />
          <feBlend
            in2="BackgroundImageFix"
            mode="normal"
            result="effect1_dropShadow_560_2659"
          />
          <feBlend
            in="SourceGraphic"
            in2="effect1_dropShadow_560_2659"
            mode="normal"
            result="shape"
          />
        </filter>
      </defs>
    </svg>
  )
}
