import { Flex, Text, VStack } from '@devup-ui/react'

import { BenchBox } from './BenchBox'
import { DevupUICard } from './DevupUICard'
import { OtherCard } from './OtherCard'

const OTHER_CARDS = [
  {
    title: 'Chakra UI',
    version: '3.37.0',
    buildTime: '19.98s',
    buildSize: '197.02MB',
    url: 'https://chakra-ui.com',
  },
  {
    title: 'Mui',
    version: '9.4.0',
    buildTime: '13.78s',
    buildSize: '95.94MB',
    url: 'https://mui.com',
  },
  {
    title: 'Kuma UI',
    version: '1.6.4',
    buildTime: '13.07s',
    buildSize: '71.28MB',
    url: 'https://kuma-ui.com',
  },
  {
    title: 'Tailwindcss',
    version: '4.3.3',
    buildTime: '12.81s',
    buildSize: '63.37MB',
    url: 'https://tailwindcss.com',
  },
  {
    title: 'panda CSS',
    version: '1.12.0',
    buildTime: '13.20s',
    buildSize: '67.68MB',
    url: 'https://panda-css.com',
  },
  {
    title: 'styleX',
    version: '0.19.0',
    buildTime: '27.61s',
    buildSize: '91.00MB',
    url: 'https://stylexjs.com',
  },
  {
    title: 'vanilla extract',
    version: '1.21.2',
    buildTime: '11.96s',
    buildSize: '64.55MB',
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
            Next.js 16.3.3 cold build (GitHub Actions, run 33254030962)
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
