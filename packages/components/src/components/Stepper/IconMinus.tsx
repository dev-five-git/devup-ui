import { SVGProps } from 'react'

export function IconMinus({ ...props }: SVGProps<SVGSVGElement>) {
  return (
    <svg
      fill="none"
      height="28"
      viewBox="0 0 28 28"
      width="28"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        clipRule="evenodd"
        d="M9 14C9 13.4477 9.3731 13 9.83333 13H18.1667C18.6269 13 19 13.4477 19 14C19 14.5523 18.6269 15 18.1667 15H9.83333C9.3731 15 9 14.5523 9 14Z"
        fill="currentColor"
        fillRule="evenodd"
      />
    </svg>
  )
}
