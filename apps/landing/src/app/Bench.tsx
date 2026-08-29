import { Flex, Text, VStack } from '@devup-ui/react'

import { BenchBox } from './BenchBox'
import { DevupUICard } from './DevupUICard'
import { OtherCard } from './OtherCard'

const OTHER_CARDS = [
  {
    title: 'Chakra UI',
    version: '3.37.0',
    buildTime: '24.79s',
    buildSize: '197.03MB',
    url: 'https://chakra-ui.com',
  },
  {
    title: 'Mui',
    version: '9.4.0',
    buildTime: '17.12s',
    buildSize: '95.96MB',
    url: 'https://mui.com',
  },
  {
    title: 'Kuma UI',
    version: '1.6.4',
    buildTime: '16.43s',
    buildSize: '71.31MB',
    url: 'https://kuma-ui.com',
  },
  {
    title: 'Tailwindcss',
    version: '4.3.3',
    buildTime: '15.55s',
    buildSize: '63.40MB',
    url: 'https://tailwindcss.com',
  },
  {
    title: 'panda CSS',
    version: '1.12.0',
    buildTime: '16.82s',
    buildSize: '67.70MB',
    url: 'https://panda-css.com',
  },
  {
    title: 'styleX',
    version: '0.19.0',
    buildTime: '34.27s',
    buildSize: '91.00MB',
    url: 'https://stylexjs.com',
  },
  {
    title: 'vanilla extract',
    version: '1.21.2',
    buildTime: '14.91s',
    buildSize: '64.57MB',
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
          <Text as="h2" color="$title" m="0" typography="h4">
            Comparison Benchmarks
          </Text>
          <Text color="$text" typography="textL">
            Next.js cold build (GitHub Actions, run 33239265133)
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
