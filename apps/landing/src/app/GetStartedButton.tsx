import { Box, css, Flex, Image, Text } from '@devup-ui/react'
import type { Locale } from 'next-intl'
import { getTranslations } from 'next-intl/server'

import { Link } from '@/i18n/navigation'

export async function GetStartedButton({
  locale = 'en',
}: {
  locale?: Locale
}) {
  const t = await getTranslations({ locale, namespace: 'Home' })
  return (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href="/docs/overview"
    >
      <Flex
        _active={{
          bg: '$negativeBase',
        }}
        _hover={{
          bg: '$title',
        }}
        alignItems="center"
        bg="$text"
        borderRadius="100px"
        data-group
        gap="20px"
        p="16px 40px"
        w="fit-content"
      >
        <Box
          _groupActive={{
            bg: '$third',
          }}
          _groupHover={{
            bg: '$primary',
          }}
          bg="$secondary"
          borderRadius="100%"
          boxSize="10px"
        />
        <Flex alignItems="center" gap="10px">
          <Text color="$base" typography="buttonL">
            {t('getStarted')}
          </Text>
          <Image bg="$base" boxSize="24px" maskImage="url(/arrow.svg)" />
        </Flex>
      </Flex>
    </Link>
  )
}
