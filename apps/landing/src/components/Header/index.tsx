import { css, Flex } from '@devup-ui/react'
import Link from 'next/link'

import { URL_PREFIX } from '../../constants'
import { Logo } from '../Logo'
import { Discord } from './Discord'
import { Github } from './Github'
import { HeaderInput } from './HeaderInput'
import { HeaderInputWrap } from './HeaderInputWrap'
import { HeaderWrap } from './HeaderWrap'
import { Menu } from './Menu'
import { MobMenu } from './MobMenu'
import { MobMenuButton } from './MobMenuButton'
import { MobMenuWrapper } from './MobMenuWrapper'
import { ThemeSwitch } from './ThemeSwitch'

export function Header() {
  const top = (
    <Flex alignItems="center">
      <Flex alignItems="center" px="10px">
        <Github />
      </Flex>
      <Flex alignItems="center" px="10px">
        <Discord />
      </Flex>
      <Flex alignItems="center" px="10px">
        <ThemeSwitch />
      </Flex>
    </Flex>
  )
  return (
    <HeaderWrap>
      <Flex alignItems="center" gap="16px">
        <MobMenuWrapper openChildren={top}>
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href={URL_PREFIX + '/'}
          >
            <Logo />
          </Link>
        </MobMenuWrapper>
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
        {top}
      </Flex>
      <Flex
        alignItems="center"
        cursor="pointer"
        display={['flex', null, 'none']}
        gap="10px"
        p="10px"
      >
        <MobMenuButton>
          <MobMenu />
        </MobMenuButton>
      </Flex>
    </HeaderWrap>
  )
}
