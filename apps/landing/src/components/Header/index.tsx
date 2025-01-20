import { css, Flex, Image, Text } from '@devup-ui/react'
import Link from 'next/link'

import { IMAGE_PREFIX } from '../../constants'
import { HeaderWrap } from './HeaderWrap'

export function Header() {
  return (
    <HeaderWrap>
      <Flex alignItems="center" gap="16px">
        <Link
          className={css`
            text-decoration: none;
          `}
          href="/"
        >
          <Image h="42px" src={IMAGE_PREFIX + '/logo.svg'} />
        </Link>
      </Flex>
      <Flex alignItems="center" gap="10px">
        <Flex alignItems="center" px="24px">
          <Link
            className={css`
              text-decoration: none;
            `}
            href="/docs/overview"
          >
            <Text color="$title" opacity="0.6" typography="buttonLsemiB">
              Docs
            </Text>
          </Link>
        </Flex>
        <Flex alignItems="center" px="24px">
          <Link
            className={css`
              text-decoration: none;
            `}
            href="/team"
          >
            <Text color="$title" opacity="0.6" typography="buttonLsemiB">
              Team
            </Text>
          </Link>
        </Flex>
        <Flex alignItems="center">
          <Flex alignItems="center" px="10px">
            <Link
              className={css`
                text-decoration: none;
              `}
              href="https://github.com/dev-five-git/devup-ui"
              target="_blank"
            >
              <Image boxSize="24px" src={IMAGE_PREFIX + '/github.svg'} />
            </Link>
          </Flex>
          <Flex alignItems="center" px="10px">
            <Link
              className={css`
                text-decoration: none;
              `}
              href="https://discord.gg/BtNffusw"
              target="_blank"
            >
              <Image boxSize="24px" src={IMAGE_PREFIX + '/discord.svg'} />
            </Link>
          </Flex>
          <Flex alignItems="center" px="10px">
            <Image boxSize="24px" src={IMAGE_PREFIX + '/light.svg'} />
          </Flex>
        </Flex>
      </Flex>
    </HeaderWrap>
  )
}
