import { css } from '@devup-ui/react'
import clsx from 'clsx'
import { ComponentProps } from 'react'

export function IconArrow({ className, ...props }: ComponentProps<'svg'>) {
  return (
    <svg
      className={clsx(css({ color: '#8b8e9c', styleOrder: 1 }), className)}
      fill="none"
      height="16"
      viewBox="0 0 16 16"
      width="16"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        clipRule="evenodd"
        d="M6.1953 12.4714C5.93495 12.211 5.93495 11.7889 6.1953 11.5286L9.7239 7.99996L6.1953 4.47136C5.93495 4.21102 5.93495 3.78891 6.1953 3.52856C6.45565 3.26821 6.87776 3.26821 7.13811 3.52856L11.1381 7.52856C11.3985 7.7889 11.3985 8.21101 11.1381 8.47136L7.13811 12.4714C6.87776 12.7317 6.45565 12.7317 6.1953 12.4714Z"
        fill="currentColor"
        fillRule="evenodd"
      />
    </svg>
  )
}
