import { Flex, Text, VStack } from '@devup-ui/react'

import { DevupUICard } from './DevupUICard'
import { OtherCard } from './OtherCard'

export function Bench() {
  return (
    <VStack alignItems="center" py="60px" w="100%">
      <VStack gap="30px" maxW="1440px" px="40px" w="100%">
        <VStack gap="16px" w="805px">
          <Text color="$title" typography="h4">
            Comparison Bechmarks
          </Text>
          <Text color="$text" typography="textL">
            Next.js Build Time and Build Size (github action - ubuntu-latest)
          </Text>
        </VStack>
        <Flex
          alignItems="end"
          flexWrap="wrap"
          gap="20px"
          justifyContent="center"
        >
          <DevupUICard />
          {[
            {
              title: 'tailwindcss',
              version: '4.1.13',
              buildTime: '20.22s',
              buildSize: '57,415kB',
              url: 'https://tailwindcss.com',
            },
            {
              title: 'styleX',
              version: '0.15.4',
              buildTime: '38.97s',
              buildSize: '76,257kB',
              url: 'https://stylexjs.com',
            },
            {
              title: 'vanilla-extract',
              version: '1.17.4',
              buildTime: '20.09s',
              buildSize: '59,366kB',
              url: 'https://vanilla-extract.style',
            },
            {
              title: 'kuma-ui',
              version: '1.5.9',
              buildTime: '21.61s',
              buildSize: '67,422kB',
              url: 'https://kuma-ui.com',
            },
            {
              title: 'panda-css',
              version: '1.3.1',
              buildTime: '22.01s',
              buildSize: '62,431kB',
              url: 'https://panda-css.com',
            },
            {
              title: 'chakra-ui',
              version: '3.27.0',
              buildTime: '29.99s',
              buildSize: '210,122kB',
              url: 'https://chakra-ui.com',
            },
            {
              title: 'mui',
              version: '7.3.2',
              buildTime: '22.21s',
              buildSize: '94,231kB',
              url: 'https://mui.com',
            },
          ].map((item) => (
            <OtherCard key={item.title} {...item} />
          ))}
        </Flex>
      </VStack>
    </VStack>
  )
}
