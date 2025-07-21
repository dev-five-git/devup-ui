'use client'

import { useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconColorPickerComp({
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
        height="95"
        rx="7.5"
        stroke={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        width="99"
        x="79.5"
        y="22.5"
      />
      <g clipPath="url(#clip0_560_2665)">
        <rect
          fill="url(#paint0_linear_560_2665)"
          height="86"
          rx="4"
          width="90"
          x="84"
          y="27"
        />
        <rect
          fill="url(#paint1_linear_560_2665)"
          height="86"
          rx="4"
          style={{ mixBlendMode: 'multiply' }}
          width="90"
          x="84"
          y="27"
        />
        <g filter="url(#filter0_d_560_2665)">
          <rect fill="#774CC5" height="10" rx="5" width="10" x="145" y="41" />
          <rect
            height="10"
            rx="5"
            stroke="white"
            strokeWidth="2"
            width="10"
            x="145"
            y="41"
          />
        </g>
      </g>
      <defs>
        <filter
          colorInterpolationFilters="sRGB"
          filterUnits="userSpaceOnUse"
          height="20"
          id="filter0_d_560_2665"
          width="20"
          x="140"
          y="36"
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
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.25 0"
          />
          <feBlend
            in2="BackgroundImageFix"
            mode="normal"
            result="effect1_dropShadow_560_2665"
          />
          <feBlend
            in="SourceGraphic"
            in2="effect1_dropShadow_560_2665"
            mode="normal"
            result="shape"
          />
        </filter>
        <linearGradient
          gradientUnits="userSpaceOnUse"
          id="paint0_linear_560_2665"
          x1="129"
          x2="129"
          y1="27"
          y2="113"
        >
          <stop stopColor="white" />
          <stop offset="1" />
        </linearGradient>
        <linearGradient
          gradientUnits="userSpaceOnUse"
          id="paint1_linear_560_2665"
          x1="84"
          x2="174"
          y1="70"
          y2="70"
        >
          <stop stopColor="white" />
          <stop offset="1" stopColor="#5A05F5" />
        </linearGradient>
        <clipPath id="clip0_560_2665">
          <rect fill="white" height="86" rx="4" width="90" x="84" y="27" />
        </clipPath>
      </defs>
    </svg>
  )
}
