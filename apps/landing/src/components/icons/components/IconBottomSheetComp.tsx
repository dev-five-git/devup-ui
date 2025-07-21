'use client'

import { useTheme } from '@devup-ui/react'
import { SVGProps } from 'react'

export default function IconBottomSheetComp({
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
      <mask fill="white" id="path-1-inside-1_560_2794">
        <path d="M43 53C43 48.5817 46.5817 45 51 45H209C213.418 45 217 48.5817 217 53V140H43V53Z" />
      </mask>
      <path
        d="M42 53C42 48.0294 46.0294 44 51 44H209C213.971 44 218 48.0294 218 53H216C216 49.134 212.866 46 209 46H51C47.134 46 44 49.134 44 53H42ZM217 140H43H217ZM42 140V53C42 48.0294 46.0294 44 51 44V46C47.134 46 44 49.134 44 53V140H42ZM209 44C213.971 44 218 48.0294 218 53V140H216V53C216 49.134 212.866 46 209 46V44Z"
        fill={theme === 'light' ? '#C8C7D1' : '#4D4C53'}
        mask="url(#path-1-inside-1_560_2794)"
      />
      <rect
        fill={theme === 'light' ? '#C3C3C3' : '#575757'}
        height="4"
        rx="2"
        width="40"
        x="110"
        y="53"
      />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
        height="30"
        rx="5"
        width="130"
        x="65"
        y="78"
      />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
        height="7"
        rx="2"
        width="91"
        x="65"
        y="114"
      />
      <rect
        fill={theme === 'light' ? '#ECECEC' : '#3a3a3a'}
        height="7"
        rx="2"
        width="91"
        x="65"
        y="125"
      />
    </svg>
  )
}
