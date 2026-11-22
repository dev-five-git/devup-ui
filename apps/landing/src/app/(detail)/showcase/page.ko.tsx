import { Text, VStack } from '@devup-ui/react'
import type { Metadata } from 'next'

import { SHOWCASE_LIST } from '@/components/showcase/showcase-list'
import { ShowcaseContent } from '@/components/showcase/ShowcaseContent'

export const metadata: Metadata = {
  title: 'Devup UI 쇼케이스',
  description: 'Devup UI로 만든 웹사이트와 앱을 소개합니다.',
  alternates: {
    canonical: '/ko/showcase',
    languages: {
      en: '/showcase',
      ko: '/ko/showcase',
      'x-default': '/showcase',
    },
  },
  openGraph: {
    title: 'Devup UI 쇼케이스',
    description: 'Devup UI로 만든 웹사이트와 앱을 소개합니다.',
    url: '/ko/showcase',
    siteName: 'Devup UI',
    images: ['/og-image.png'],
    locale: 'ko_KR',
  },
}

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
        <Text as="h1" color="$title" m="0" typography="h4">
          쇼케이스
        </Text>
        <Text color="$text" typography="bodyReg">
          Devup UI로 만든 다양한 웹사이트를 소개합니다
        </Text>
      </VStack>
      <ShowcaseContent list={SHOWCASE_LIST} />
    </VStack>
  )
}
