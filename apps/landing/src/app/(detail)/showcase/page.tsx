import { Text, VStack } from '@devup-ui/react'

import { ShowcaseContent } from '@/components/showcase/ShowcaseContent'

const SHOWCASE_LIST = [
  {
    id: 'devfive',
    name: 'Devfive',
    image: '/showcase/devfive.webp',
    type: 'website',
    link: 'https://devfive.kr',
  },
  {
    id: 'braillify',
    name: 'Braillify',
    image: '/showcase/braillify.webp',
    type: 'website',
    link: 'https://braillify.kr',
  },
  {
    id: 'ideocean',
    name: 'IDeOcean',
    image: '/showcase/ideocean.webp',
    type: 'website',
    link: 'https://ideaocean.ai/',
  },
  {
    id: 'ongle',
    name: 'Ongle',
    image: '/showcase/ongle.webp',
    type: 'website',
    link: 'https://easy-read.co.kr',
  },
  {
    id: 'laon_singcraft',
    name: 'Laon Singcraft',
    image: '/showcase/laon_singcraft.webp',
    type: 'website',
    link: 'https://laonswingcraft.com/',
  },
  {
    id: 'point_park',
    name: 'PointPark',
    image: '/showcase/point_park.webp',
    type: 'mobile',
    link: 'https://play.google.com/store/apps/details?id=com.pointpark.popaArdApp&hl=ko',
  },
  {
    id: 'nh_allone_bank',
    name: 'NH Allone Bank',
    image: '/showcase/nh_allone_bank.webp',
    type: 'mobile',
    link: 'https://play.google.com/store/apps/details?id=com.nonghyup.nhallonebank',
  },
  {
    id: 'your_test',
    name: 'Your Test',
    image: '/showcase/your_test.webp',
    type: 'website',
    link: 'https://www.yourtest.kr/',
  },
  {
    id: 'util_support',
    name: 'Util Support',
    image: '/showcase/util_support.webp',
    type: 'website',
    link: 'https://util.support/',
  },
  {
    id: 'justq',
    name: 'Justq',
    image: '/showcase/justq.webp',
    type: 'website',
    link: 'https://www.justq.com/',
  },
  {
    id: 'kavia',
    name: 'Kavia',
    image: '/showcase/kavia.webp',
    type: 'website',
    link: 'https://kavia.org/',
  },
  {
    id: 'boratial',
    name: 'Boratial',
    image: '/showcase/boratial.webp',
    type: 'website',
    link: 'https://www.boratr.co.kr',
  },
]

export default function Page() {
  return (
    <VStack
      gap="30px"
      maxW="1032px"
      mx="auto"
      overflow="hidden"
      px={['20px', '30px', null, '40px', '60px']}
      py="40px"
      w="100%"
    >
      <VStack gap="12px">
        <Text color="$title" typography="h4">
          Showcase
        </Text>
        <Text color="$text" typography="bodyReg">
          Showcasing a variety of websites built with Devup UI
        </Text>
      </VStack>
      <ShowcaseContent list={SHOWCASE_LIST} />
    </VStack>
  )
}
