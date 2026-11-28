import { Text, VStack } from '@devup-ui/react'
import type { Metadata } from 'next'

import { SHOWCASE_LIST } from '@/components/showcase/showcase-list'
import { ShowcaseContent } from '@/components/showcase/ShowcaseContent'

export const metadata: Metadata = {
  title: 'Devup UI Showcase',
  description: 'Websites and apps built with Devup UI.',
  alternates: {
    canonical: '/showcase',
    languages: {
      en: '/showcase',
      ko: '/ko/showcase',
      'x-default': '/showcase',
    },
  },
  openGraph: {
    title: 'Devup UI Showcase',
    description: 'Websites and apps built with Devup UI.',
    url: '/showcase',
    siteName: 'Devup UI',
    images: ['/og-image.png'],
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
