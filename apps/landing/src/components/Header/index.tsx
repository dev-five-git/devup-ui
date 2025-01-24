import { css, Flex } from '@devup-ui/react'
import Link from 'next/link'

import { URL_PREFIX } from '../../constants'
import { Logo } from '../Logo'
import { HeaderInput } from './HeaderInput'
import { HeaderInputWrap } from './HeaderInputWrap'
import { HeaderWrap } from './HeaderWrap'
import { Menu } from './Menu'
import { ThemeSwitch } from './ThemeSwitch'

export function Header() {
  return (
    <HeaderWrap>
      <Flex alignItems="center" gap="16px">
        <Link
          className={css({
            textDecoration: 'none',
          })}
          href={URL_PREFIX + '/'}
        >
          <Logo />
        </Link>
      </Flex>
      <Flex alignItems="center" display={['none', null, 'flex']} gap="10px">
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href={URL_PREFIX + '/docs/overview'}
          >
            <Menu keyword="docs">Docs</Menu>
          </Link>
        </Flex>
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href={URL_PREFIX + '/team'}
          >
            <Menu keyword="team">Team</Menu>
          </Link>
        </Flex>
        <HeaderInputWrap>
          <HeaderInput />
        </HeaderInputWrap>
        <Flex alignItems="center">
          <Flex alignItems="center" px="10px">
            <Link
              className={css({
                textDecoration: 'none',
              })}
              href="https://github.com/dev-five-git/devup-ui"
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
                  d="M12 2C10.6868 2 9.38642 2.25866 8.17317 2.7612C6.95991 3.26375 5.85752 4.00035 4.92893 4.92893C3.05357 6.8043 2 9.34784 2 12C2 16.42 4.87 20.17 8.84 21.5C9.34 21.58 9.5 21.27 9.5 21V19.31C6.73 19.91 6.14 17.97 6.14 17.97C5.68 16.81 5.03 16.5 5.03 16.5C4.12 15.88 5.1 15.9 5.1 15.9C6.1 15.97 6.63 16.93 6.63 16.93C7.5 18.45 8.97 18 9.54 17.76C9.63 17.11 9.89 16.67 10.17 16.42C7.95 16.17 5.62 15.31 5.62 11.5C5.62 10.39 6 9.5 6.65 8.79C6.55 8.54 6.2 7.5 6.75 6.15C6.75 6.15 7.59 5.88 9.5 7.17C10.29 6.95 11.15 6.84 12 6.84C12.85 6.84 13.71 6.95 14.5 7.17C16.41 5.88 17.25 6.15 17.25 6.15C17.8 7.5 17.45 8.54 17.35 8.79C18 9.5 18.38 10.39 18.38 11.5C18.38 15.32 16.04 16.16 13.81 16.41C14.17 16.72 14.5 17.33 14.5 18.26V21C14.5 21.27 14.66 21.59 15.17 21.5C19.14 20.16 22 16.42 22 12C22 10.6868 21.7413 9.38642 21.2388 8.17317C20.7362 6.95991 19.9997 5.85752 19.0711 4.92893C18.1425 4.00035 17.0401 3.26375 15.8268 2.7612C14.6136 2.25866 13.3132 2 12 2Z"
                  fill="currentColor"
                />
              </svg>
            </Link>
          </Flex>
          <Flex alignItems="center" px="10px">
            <Link
              className={css({
                textDecoration: 'none',
              })}
              href="https://discord.gg/BtNffusw"
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
                  d="M19.2701 5.33C17.9401 4.71 16.5001 4.26 15.0001 4C14.9737 4.00038 14.9486 4.01116 14.9301 4.03C14.7501 4.36 14.5401 4.79 14.4001 5.12C12.8091 4.88015 11.1911 4.88015 9.60012 5.12C9.46012 4.78 9.25012 4.36 9.06012 4.03C9.05012 4.01 9.02012 4 8.99012 4C7.49012 4.26 6.06012 4.71 4.72012 5.33C4.71012 5.33 4.70012 5.34 4.69012 5.35C1.97012 9.42 1.22012 13.38 1.59012 17.3C1.59012 17.32 1.60012 17.34 1.62012 17.35C3.42012 18.67 5.15012 19.47 6.86012 20C6.89012 20.01 6.92012 20 6.93012 19.98C7.33012 19.43 7.69012 18.85 8.00012 18.24C8.02012 18.2 8.00012 18.16 7.96012 18.15C7.39012 17.93 6.85012 17.67 6.32012 17.37C6.28012 17.35 6.28012 17.29 6.31012 17.26C6.42012 17.18 6.53012 17.09 6.64012 17.01C6.66012 16.99 6.69012 16.99 6.71012 17C10.1501 18.57 13.8601 18.57 17.2601 17C17.2801 16.99 17.3101 16.99 17.3301 17.01C17.4401 17.1 17.5501 17.18 17.6601 17.27C17.7001 17.3 17.7001 17.36 17.6501 17.38C17.1301 17.69 16.5801 17.94 16.0101 18.16C15.9701 18.17 15.9601 18.22 15.9701 18.25C16.2901 18.86 16.6501 19.44 17.0401 19.99C17.0701 20 17.1001 20.01 17.1301 20C18.8501 19.47 20.5801 18.67 22.3801 17.35C22.4001 17.34 22.4101 17.32 22.4101 17.3C22.8501 12.77 21.6801 8.84 19.3101 5.35C19.3001 5.34 19.2901 5.33 19.2701 5.33ZM8.52012 14.91C7.49012 14.91 6.63012 13.96 6.63012 12.79C6.63012 11.62 7.47012 10.67 8.52012 10.67C9.58012 10.67 10.4201 11.63 10.4101 12.79C10.4101 13.96 9.57012 14.91 8.52012 14.91ZM15.4901 14.91C14.4601 14.91 13.6001 13.96 13.6001 12.79C13.6001 11.62 14.4401 10.67 15.4901 10.67C16.5501 10.67 17.3901 11.63 17.3801 12.79C17.3801 13.96 16.5501 14.91 15.4901 14.91Z"
                  fill="currentColor"
                />
              </svg>
            </Link>
          </Flex>
          <Flex alignItems="center" px="10px">
            <ThemeSwitch />
          </Flex>
        </Flex>
      </Flex>
      <Flex
        alignItems="center"
        cursor="pointer"
        display={['flex', null, 'none']}
        gap="10px"
        p="10px"
      >
        <svg
          fill="none"
          height="32"
          viewBox="0 0 32 32"
          width="32"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            clipRule="evenodd"
            d="M5.33325 9.33333C5.33325 8.59695 5.93021 8 6.66659 8H25.3333C26.0696 8 26.6666 8.59695 26.6666 9.33333C26.6666 10.0697 26.0696 10.6667 25.3333 10.6667H6.66659C5.93021 10.6667 5.33325 10.0697 5.33325 9.33333ZM5.33325 16C5.33325 15.2636 5.93021 14.6667 6.66659 14.6667H25.3333C26.0696 14.6667 26.6666 15.2636 26.6666 16C26.6666 16.7364 26.0696 17.3333 25.3333 17.3333H6.66659C5.93021 17.3333 5.33325 16.7364 5.33325 16ZM6.66659 21.3333C5.93021 21.3333 5.33325 21.9303 5.33325 22.6667C5.33325 23.403 5.93021 24 6.66659 24H25.3333C26.0696 24 26.6666 23.403 26.6666 22.6667C26.6666 21.9303 26.0696 21.3333 25.3333 21.3333H6.66659Z"
            fill="#2F2F2F"
            fillRule="evenodd"
          />
        </svg>
      </Flex>
    </HeaderWrap>
  )
}
