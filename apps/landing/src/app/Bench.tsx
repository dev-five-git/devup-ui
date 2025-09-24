import { Box, Flex, Text, VStack } from '@devup-ui/react'

import { DevupUICard } from './DevupUICard'
import { OtherCard } from './OtherCard'

export function Bench() {
  return (
    <VStack
      alignItems="center"
      overflow="hidden"
      py={['40px', null, '50px', null, '60px']}
      w="100%"
    >
      <VStack
        gap="30px"
        maxW="1440px"
        px={[null, null, null, null, '40px']}
        w="100%"
      >
        <VStack
          gap="16px"
          mx={[4, null, '40px', null, 0]}
          textAlign={['center', null, 'left']}
          wordBreak="keep-all"
        >
          <Text color="$title" typography="h4">
            Comparison Bechmarks
          </Text>
          <Text color="$text" typography="textL">
            Next.js Build Time and Build Size (github action - ubuntu-latest)
          </Text>
        </VStack>

        <Flex display={[null, null, null, null, 'none']}>
          <DevupUICard />
        </Flex>
        <Box
          overflow={['auto', null, null, null, 'visible']}
          scrollbarWidth="none"
        >
          <Flex
            flexWrap={[null, null, null, null, 'wrap']}
            gap={[3, null, 5]}
            justifyContent={[null, null, null, null, 'center']}
            px={[4, null, '40px', null, 0]}
          >
            <Flex display={['none', null, null, null, 'flex']}>
              <DevupUICard />
            </Flex>
            {[
              {
                title: 'Chakra UI',
                version: '3.24.2',
                buildTime: '29.3s',
                buildSize: '186.2MB',
                url: 'https://chakra-ui.com',
              },
              {
                title: 'Mui',
                version: '7.3.1',
                buildTime: '21.6s',
                buildSize: '84.3MB',
                url: 'https://mui.com',
              },
              {
                title: 'Kuma UI',
                version: '1.5.9',
                buildTime: '20.6s',
                buildSize: '60.3MB',
                url: 'https://kuma-ui.com',
              },
              {
                title: 'Tailwindcss',
                version: '4.1.13',
                buildTime: '20.2s',
                buildSize: '54.7MB',
                url: 'https://tailwindcss.com',
              },
              {
                title: 'panda CSS',
                version: '1.3.1',
                buildTime: '22.0s',
                buildSize: '59.5MB',
                url: 'https://panda-css.com',
              },
              {
                title: 'styleX',
                version: '0.15.4',
                buildTime: '38.9s',
                buildSize: '72.7MB',
                url: 'https://stylexjs.com',
              },
              {
                title: 'vanilla extract',
                version: '1.17.4',
                buildTime: '20.1s',
                buildSize: '56.6MB',
                url: 'https://vanilla-extract.style',
              },
            ].map((item) => (
              <OtherCard key={item.title} {...item} />
            ))}
          </Flex>
        </Box>
      </VStack>
    </VStack>
  )
}
