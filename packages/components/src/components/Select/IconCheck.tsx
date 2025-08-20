import { ComponentProps } from 'react'

export function IconCheck({ ...props }: ComponentProps<'svg'>) {
  return (
    <svg
      fill="none"
      height="10"
      viewBox="0 0 12 10"
      width="12"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        clipRule="evenodd"
        d="M11.6044 0.825663C12.1054 1.28367 12.1344 2.05481 11.6692 2.54805L5.4787 9.11055C5.24011 9.36348 4.90375 9.50497 4.55315 9.49987C4.20254 9.49477 3.87058 9.34356 3.63968 9.0838L0.30635 5.3338C-0.143922 4.82725 -0.0917767 4.05729 0.42282 3.61405C0.937417 3.17081 1.7196 3.22214 2.16987 3.7287L4.59876 6.4612L9.85463 0.889455C10.3199 0.396214 11.1033 0.367654 11.6044 0.825663Z"
        fill="currentColor"
        fillRule="evenodd"
      />
    </svg>
  )
}
