import { css } from '@devup-ui/react'
import Link from 'next/link'

export function Kakao() {
  return (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href="#"
      target="_blank"
    >
      <svg
        className={css({
          color: '$text',
        })}
        fill="none"
        height="24"
        viewBox="0 0 24 24"
        width="24"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          clipRule="evenodd"
          d="M12.0001 3C5.92457 3 1 6.76292 1 11.4039C1 14.2901 2.90472 16.8346 5.80521 18.348L4.58483 22.7571C4.477 23.1467 4.92752 23.4572 5.27347 23.2314L10.623 19.7396C11.0745 19.7827 11.5332 19.8078 12.0001 19.8078C18.0751 19.8078 23 16.045 23 11.4039C23 6.76292 18.0751 3 12.0001 3Z"
          fill="#1A1A1A"
          fillRule="evenodd"
        />
      </svg>
    </Link>
  )
}
