import { css, Flex, Image, Text } from '@devup-ui/react'
import Link from 'next/link'

import { URL_PREFIX } from '../../constants'
import { HeaderWrap } from './HeaderWrap'

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
          <Image h="42px" src={URL_PREFIX + '/logo.svg'} />
        </Link>
      </Flex>
      <Flex alignItems="center" gap="10px">
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href={URL_PREFIX + '/docs/overview'}
          >
            <Text color="$title" opacity="0.6" typography="buttonLsemiB">
              Docs
            </Text>
          </Link>
        </Flex>
        <Flex alignItems="center" px="24px">
          <Link
            className={css({
              textDecoration: 'none',
            })}
            href={URL_PREFIX + '/team'}
          >
            <Text color="$title" opacity="0.6" typography="buttonLsemiB">
              Team
            </Text>
          </Link>
        </Flex>
        <Flex alignItems="center">
          <Flex alignItems="center" px="10px">
            <Link
              className={css({
                textDecoration: 'none',
              })}
              href="https://github.com/dev-five-git/devup-ui"
              target="_blank"
            >
              <Image boxSize="24px" src={URL_PREFIX + '/github.svg'} />
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
              <Image boxSize="24px" src={URL_PREFIX + '/discord.svg'} />
            </Link>
          </Flex>
          <Flex alignItems="center" px="10px">
            <Image boxSize="24px" src={URL_PREFIX + '/light.svg'} />
          </Flex>
        </Flex>
      </Flex>
    </HeaderWrap>
  )
}
