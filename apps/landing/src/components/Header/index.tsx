import { Box, css, Flex } from '@devup-ui/react'
import Link from 'next/link'
import { Suspense } from 'react'

import { Logo } from '../Logo'
import { Discord } from './Discord'
import { Github } from './Github'
import { HeaderInput } from './HeaderInput'
import { HeaderInputWrap } from './HeaderInputWrap'
import { HeaderWrap } from './HeaderWrap'
import { Kakao } from './Kakao'
import { Menu } from './Menu'
import { MobMenu } from './MobMenu'
import { MobMenuButton } from './MobMenuButton'
import { MobMenuWrapper } from './MobMenuWrapper'
import { ThemeSwitch } from './ThemeSwitch'

export function Header() {
  const top = (
    <Flex alignItems="center">
      <Flex alignItems="center" px="10px">
        <Link
          className={css({
            textDecoration: 'none',
          })}
          href="https://github.com/dev-five-git/devup-ui"
          target="_blank"
        >
          <Github />
        </Link>
      </Flex>
      <Flex alignItems="center" px="10px">
        <Discord />
      </Flex>
      <Flex alignItems="center" px="10px">
        <Kakao />
      </Flex>
      <Flex alignItems="center" px="10px">
        <ThemeSwitch />
      </Flex>
    </Flex>
  )
  return (
    <HeaderWrap>
      <Flex alignItems="center" gap="16px">
        <Box display={['none', null, 'contents']}>
          <Link
            className={css({
              textDecoration: 'none',
              ml: 4,
            })}
            href="/"
          >
            <Logo />
          </Link>
        </Box>
        <Box display={['contents', null, 'none']}>
          <Suspense
            fallback={
              <Link
                className={css({
                  textDecoration: 'none',
                  ml: 4,
                })}
                href="/"
              >
                <Logo />
              </Link>
            }
          >
            <MobMenuWrapper openChildren={top}>
              <Link
                className={css({
                  textDecoration: 'none',
                  ml: 4,
                })}
                href="/"
              >
                <Logo />
              </Link>
            </MobMenuWrapper>
          </Suspense>
        </Box>
      </Flex>
      <Flex alignItems="center" display={['none', null, 'flex']} gap="10px">
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href="/docs/overview"
          >
            <Menu keyword="docs">Docs</Menu>
          </Link>
        </Flex>
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href="/components/overview"
          >
            <Menu keyword="components">Components</Menu>
          </Link>
        </Flex>
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href="/team"
          >
            <Menu keyword="team">Team</Menu>
          </Link>
        </Flex>
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
              display: 'flex',
              gap: '4px',
              alignItems: 'center',
            })}
            href="/storybook/index.html"
            role="group"
          >
            <Menu keyword="team">Storybook</Menu>
            <Box
              _groupActive={{
                opacity: '1',
                bg: '$primary',
              }}
              _groupHover={{
                opacity: '1',
              }}
              bg="$text"
              boxSize="24px"
              maskImage="url(/outlink.svg)"
              maskPosition="center"
              maskRepeat="no-repeat"
              maskSize="contain"
              opacity="0.6"
            />
          </Link>
        </Flex>
        <HeaderInputWrap>
          <HeaderInput readOnly />
        </HeaderInputWrap>
        {top}
      </Flex>
      <Flex
        alignItems="center"
        cursor="pointer"
        display={['flex', null, 'none']}
        gap="10px"
        p="10px"
      >
        <Suspense>
          <MobMenuButton>
            <MobMenu />
          </MobMenuButton>
        </Suspense>
      </Flex>
    </HeaderWrap>
  )
}
