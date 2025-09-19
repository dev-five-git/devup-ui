'use client'

import { css } from '@devup-ui/react'
import { useRouter, useSearchParams } from 'next/navigation'

interface MobMenuButtonProps {
  children: React.ReactNode
}

export function MobMenuButton({ children }: MobMenuButtonProps) {
  const { replace } = useRouter()
  const menu = useSearchParams().get('menu') === '1'
  return (
    <>
      <svg
        aria-label="Menu Button"
        className={css({
          color: '$text',
        })}
        fill="none"
        height="32"
        onClick={() => replace(`?menu=${menu ? '0' : '1'}`)}
        viewBox="0 0 32 32"
        width="32"
        xmlns="http://www.w3.org/2000/svg"
      >
        {menu ? (
          <path
            clipRule="evenodd"
            d="M24.9431 8.9428C25.4638 8.4221 25.4638 7.57788 24.9431 7.05718C24.4224 6.53648 23.5782 6.53648 23.0575 7.05718L16.0003 14.1144L8.94313 7.05718C8.42244 6.53648 7.57822 6.53648 7.05752 7.05718C6.53682 7.57788 6.53682 8.4221 7.05752 8.9428L14.1147 16L7.05752 23.0572C6.53682 23.5779 6.53682 24.4221 7.05752 24.9428C7.57822 25.4635 8.42244 25.4635 8.94313 24.9428L16.0003 17.8856L23.0575 24.9428C23.5782 25.4635 24.4224 25.4635 24.9431 24.9428C25.4638 24.4221 25.4638 23.5779 24.9431 23.0572L17.8859 16L24.9431 8.9428Z"
            fill="currentColor"
            fillRule="evenodd"
          />
        ) : (
          <path
            clipRule="evenodd"
            d="M5.33325 9.33333C5.33325 8.59695 5.93021 8 6.66659 8H25.3333C26.0696 8 26.6666 8.59695 26.6666 9.33333C26.6666 10.0697 26.0696 10.6667 25.3333 10.6667H6.66659C5.93021 10.6667 5.33325 10.0697 5.33325 9.33333ZM5.33325 16C5.33325 15.2636 5.93021 14.6667 6.66659 14.6667H25.3333C26.0696 14.6667 26.6666 15.2636 26.6666 16C26.6666 16.7364 26.0696 17.3333 25.3333 17.3333H6.66659C5.93021 17.3333 5.33325 16.7364 5.33325 16ZM6.66659 21.3333C5.93021 21.3333 5.33325 21.9303 5.33325 22.6667C5.33325 23.403 5.93021 24 6.66659 24H25.3333C26.0696 24 26.6666 23.403 26.6666 22.6667C26.6666 21.9303 26.0696 21.3333 25.3333 21.3333H6.66659Z"
            fill="currentColor"
            fillRule="evenodd"
          />
        )}
      </svg>
      {menu && children}
    </>
  )
}
