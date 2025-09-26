import { Flex, Text, VStack } from '@devup-ui/react'

import { BenchBox } from './BenchBox'
import { DevupUICard } from './DevupUICard'
import { OtherCard } from './OtherCard'

const OTHER_CARDS = [
  {
    title: 'Chakra UI',
    version: '3.27.0',
    buildTime: '29.99s',
    buildSize: '200.4MB',
    url: 'https://chakra-ui.com',
  },
  {
    title: 'Mui',
    version: '7.3.2',
    buildTime: '22.21s',
    buildSize: '89.9MB',
    url: 'https://mui.com',
  },
  {
    title: 'Kuma UI',
    version: '1.5.9',
    buildTime: '21.61s',
    buildSize: '64.3MB',
    url: 'https://kuma-ui.com',
  },
  {
    title: 'Tailwindcss',
    version: '4.1.13',
    buildTime: '20.22s',
    buildSize: '54.8MB',
    url: 'https://tailwindcss.com',
  },
  {
    title: 'panda CSS',
    version: '1.3.1',
    buildTime: '22.01s',
    buildSize: '59.5MB',
    url: 'https://panda-css.com',
  },
  {
    title: 'styleX',
    version: '0.15.4',
    buildTime: '38.97s',
    buildSize: '54.7MB',
    url: 'https://stylexjs.com',
  },
  {
    title: 'vanilla extract',
    version: '1.17.4',
    buildTime: '20.09s',
    buildSize: '56.6MB',
    url: 'https://vanilla-extract.style',
  },
]

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
        <BenchBox>
          <Flex
            alignItems="flex-end"
            flexWrap={[null, null, null, null, 'wrap']}
            gap={[3, null, 5]}
            justifyContent={[null, null, null, null, 'center']}
            px={[4, null, '40px', null, 0]}
            w="fit-content"
          >
            <Flex display={['none', null, null, null, 'flex']}>
              <DevupUICard />
            </Flex>
            {OTHER_CARDS.map((item) => (
              <OtherCard key={item.title} {...item} />
            ))}
          </Flex>
        </BenchBox>
      </VStack>
    </VStack>
  )
}
