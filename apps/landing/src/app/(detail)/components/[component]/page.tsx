import { Box, Flex, Text, VStack } from '@devup-ui/react'

import { CustomCode } from '@/components/mdx/components/CustomCode'
import { CustomH4 } from '@/components/mdx/components/CustomH4'
import { CustomH6 } from '@/components/mdx/components/CustomH6'
import { CustomParagraph } from '@/components/mdx/components/CustomParagraph'
import { CustomPre } from '@/components/mdx/components/CustomPre'
import { CustomStrong } from '@/components/mdx/components/CustomStrong'
import { PropsTable } from '@/components/props-table'
import { COMPONENT_GROUPS } from '@/constants'
import { getDemos } from '@/utils/get-demos'

import MdxCard from '../MdxCard'

export const generateMetadata = async ({
  params,
}: {
  params: Promise<{ component: string }>
}) => {
  const { component } = await params
  const pascalWithSpace = component
    .split('-')
    .map((item) => item.charAt(0).toUpperCase() + item.slice(1))
    .join(' ')
  return {
    title: `Devup UI - ${pascalWithSpace}`,
    description: `${pascalWithSpace} component`,
    alternates: {
      canonical: `/components/${component}`,
    },
    openGraph: {
      title: `Devup UI - ${pascalWithSpace}`,
      description: `${pascalWithSpace} component`,
      url: `/components/${component}`,
      siteName: 'Devup UI',
      images: [`/components-og/${component}.webp`],
    },
  }
}

export const generateStaticParams = async () => {
  return Object.values(COMPONENT_GROUPS)
    .flat()
    .map((component) => ({
      component,
    }))
}

export default async function Page({
  params,
}: {
  params: Promise<{ component: string }>
}) {
  const { component } = await params
  const c = await getDemos(component)
  const m = Math.ceil(c.length / 2)
  const { default: Index } = await import(`./${component}/index.mdx`)
  const { default: Api } = await import(`./${component}/Api.mdx`)
  const componentName = component
    .split('-')
    .map((item) => item.charAt(0).toUpperCase() + item.slice(1))
    .join(' ')

  return (
    <VStack gap="16px" maxW="100%" overflow="hidden">
      <Text as="strong" color="$primary" m="0" typography="captionBold">
        {componentName}
      </Text>
      <Text as="h4" color="$title" m="0" typography="h4">
        {componentName}
      </Text>
      <Index
        components={{
          h4: CustomH4,
          h6: CustomH6,
          pre: CustomPre,
          strong: CustomStrong,
          p: CustomParagraph,
        }}
      />
      <VStack gap="16px" py="30px">
        <Text as="h6" color="$title" m="0" typography="h6">
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
            PropsTable,
          }}
        />
      </VStack>
    </VStack>
  )
}
