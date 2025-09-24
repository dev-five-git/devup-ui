import { Box, css, Flex, Grid, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'

const FEATURES = [
  {
    icon: '/idea.svg',
    title: 'Zero Runtime',
    desc: 'A futuristic design that eliminates the root causes of performance degradation.',
  },
  {
    icon: '/trophy.svg',
    title: 'Top Performance',
    desc: 'The fastest build speed and the smallest bundle size among CSS-in-JS solutions.',
  },
  {
    icon: '/heart.svg',
    title: 'Type Safety',
    desc: 'Enhanced DX with typing-based support.',
  },
  {
    icon: '/notice.svg',
    title: 'Figma Plugin',
    desc: 'A Figma plugin enabling safer and faster development.',
    extra: (
      <Box
        pos="absolute"
        right={['10px', null, null, null, '20px']}
        top={['10px', null, null, null, '20px']}
      >
        <FigmaButton />
      </Box>
    ),
  },
]

export function Feature() {
  return (
    <VStack alignItems="center" py={['30px', null, '50px']} w="100%">
      <VStack gap="40px" maxW="1440px" px={['16px', null, '40px']} w="100%">
        <VStack gap="16px">
          <Text color="$title" typography="h4">
            Features
          </Text>
          <Text color="$text" typography="textL">
            Devup UI offers a performance-optimized CSS-in-JS system, theme
            typing, <Box as="br" display={['none', null, 'initial']} />
            and amazing features for faster and safer development.
          </Text>
        </VStack>
        <Grid gap="16px" gridTemplateColumns={['1fr', null, '1fr 1fr']}>
          {FEATURES.map(({ icon, title, desc, extra }) => (
            <Flex
              key={title}
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
                <Text color="$title" typography="h6">
                  {title}
                </Text>
                <Text color="$text" typography="body">
                  {desc}
                </Text>
              </VStack>
              {extra}
            </Flex>
          ))}
        </Grid>
      </VStack>
    </VStack>
  )
}
export function Icons() {
  return (
    <Flex boxSize="24px">
      <Box
        bg="$text"
        h="16px"
        maskImage="url(/icons/Union.svg)"
        maskRepeat="no-repeat"
        maskSize="contain"
        w="16px"
      />
    </Flex>
  )
}
export function FigmaButton() {
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
            Go Figma Community
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
