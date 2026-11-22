import { Box, css, Flex, Grid, Image, Text, VStack } from '@devup-ui/react'
import type { Locale } from 'next-intl'
import { getTranslations } from 'next-intl/server'

import { Link } from '@/i18n/navigation'

const FEATURES = [
  { icon: '/idea.svg', key: 'zeroRuntime' },
  { icon: '/trophy.svg', key: 'topPerformance' },
  { icon: '/heart.svg', key: 'typeSafety' },
  { icon: '/notice.svg', key: 'figmaPlugin' },
] as const

export async function Feature({ locale = 'en' }: { locale?: Locale }) {
  const t = await getTranslations({ locale, namespace: 'Home' })
  return (
    <VStack alignItems="center" py={['30px', null, '50px']} w="100%">
      <VStack gap="40px" maxW="1440px" px={['16px', null, '40px']} w="100%">
        <VStack gap="16px">
          <Text as="h2" color="$title" m="0" typography="h4">
            {t('featuresTitle')}
          </Text>
          <Text color="$text" typography="textL">
            {t.rich('featuresDescription', {
              br: () => <Box as="br" display={['none', null, 'initial']} />,
            })}
          </Text>
        </VStack>
        <Grid gap="16px" gridTemplateColumns={['1fr', null, '1fr 1fr']}>
          {FEATURES.map(({ icon, key }) => (
            <Flex
              key={key}
              bg="$containerBackground"
              border="1px solid $border"
              borderRadius="20px"
              boxShadow="0 4px 12px 0 rgba(135, 135, 135, 0.06)"
              flex="1"
              flexDir={[null, null, 'column']}
              gap="20px"
              h="100%"
              p={['20px', null, '30px']}
              pos="relative"
            >
              <Image boxSize="32px" src={icon} />
              <VStack gap="10px">
                <Text as="h3" color="$title" m="0" typography="h6">
                  {t(`features.${key}.title`)}
                </Text>
                <Text color="$text" typography="body">
                  {t(`features.${key}.description`)}
                </Text>
              </VStack>
              {key === 'figmaPlugin' && (
                <Box
                  pos="absolute"
                  right={['10px', null, null, null, '20px']}
                  top={['10px', null, null, null, '20px']}
                >
                  <FigmaButton label={t('goFigmaCommunity')} />
                </Box>
              )}
            </Flex>
          ))}
        </Grid>
      </VStack>
    </VStack>
  )
}
function FigmaButton({ label }: { label: string }) {
  return (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href="https://www.figma.com/community/plugin/1412341601954480694"
      target="_blank"
    >
      <Box
        _active={{
          bg: '$menuActive',
        }}
        _hover={{ bg: '$menuHover' }}
        borderRadius="8px"
        p="8px"
        transition="background-color 0.2s ease"
      >
        <Flex alignItems="center" gap="4px" justifyContent="flex-end">
          <Text
            color="$primary"
            display={['none', null, 'revert']}
            typography="body"
          >
            {label}
          </Text>
          <Box
            aspectRatio="1"
            bg="$primary"
            maskImage="url(/icons/link.svg)"
            maskRepeat="no-repeat"
            maskSize="contain"
            w="16px"
          />
        </Flex>
      </Box>
    </Link>
  )
}
