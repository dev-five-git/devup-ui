import { Box, Flex, Text, VStack } from '@devup-ui/react'
import { Metadata } from 'next'

import { CustomCode } from '@/components/mdx/components/CustomCode'
import { CustomH4 } from '@/components/mdx/components/CustomH4'
import { CustomH6 } from '@/components/mdx/components/CustomH6'
import { CustomParagraph } from '@/components/mdx/components/CustomParagraph'
import { CustomPre } from '@/components/mdx/components/CustomPre'
import { CustomStrong } from '@/components/mdx/components/CustomStrong'
import { getDemos } from '@/utils/get-demos'

import MdxCard from '../MdxCard'
import Api from './Api.mdx'
import Button from './Button.mdx'

export const metadata: Metadata = {
  title: 'Devup UI - Button',
  description: 'Button component',
  alternates: {
    canonical: '/components/button',
  },
  openGraph: {
    title: 'Devup UI - Button',
    description: 'Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor',
    url: '/components/button',
    siteName: 'Devup UI',
    images: ['https://devup-ui.com/components-og/button.webp'],
  },
}

export default async function Page() {
  const c = await getDemos(__dirname.split(/[\\/]/).pop()!)
  const m = Math.ceil(c.length / 2)

  return (
    <VStack gap="16px" maxW="100%" overflow="hidden">
      <Button
        components={{
          h4: CustomH4,
          h6: CustomH6,
          pre: CustomPre,
          strong: CustomStrong,
          p: CustomParagraph,
        }}
      />
      <VStack gap="16px" py="30px">
        <Text color="$title" typography="h6">
          Examples
        </Text>
        <Flex flexWrap="wrap" gap="16px">
          <Box flex="1" minW="300px" w="calc(50% - 8px)">
            {c.slice(0, m).map(([Demo, src]) => (
              <MdxCard key={src} demo={<Demo />} src={src} />
            ))}
          </Box>
          <Box flex="1" minW="300px" w="calc(50% - 8px)">
            {c.slice(m).map(([Demo, src]) => (
              <MdxCard key={src} demo={<Demo />} src={src} />
            ))}
          </Box>
        </Flex>
      </VStack>
      <VStack gap="16px" py="30px">
        <Api
          components={{
            code: CustomCode,
            h4: CustomH4,
            h6: CustomH6,
            pre: CustomPre,
            p: CustomParagraph,
          }}
        />
      </VStack>
    </VStack>
  )
}
