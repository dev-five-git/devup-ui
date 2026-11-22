import { Box, Flex, Text, VStack } from '@devup-ui/react'
import type { MDXProps } from 'mdx/types'
import type { Metadata } from 'next'
import { createTranslator, type Locale } from 'next-intl'
import { getTranslations } from 'next-intl/server'
import type { ComponentType } from 'react'

import { CustomCode } from '@/components/mdx/components/CustomCode'
import { CustomH4 } from '@/components/mdx/components/CustomH4'
import { CustomH6 } from '@/components/mdx/components/CustomH6'
import { CustomParagraph } from '@/components/mdx/components/CustomParagraph'
import { CustomPre } from '@/components/mdx/components/CustomPre'
import { CustomStrong } from '@/components/mdx/components/CustomStrong'
import { PropsTable } from '@/components/PropsTable'
import { getLocaleMessages } from '@/i18n/messages'
import { getDemos } from '@/utils/get-demos'

import MdxCard from './MdxCard'

/** `date-picker` -> `Date Picker`; the component name stays English. */
export function getComponentName(component: string) {
  return component
    .split('-')
    .map((item) => item.charAt(0).toUpperCase() + item.slice(1))
    .join(' ')
}

export function getComponentMetadata(
  component: string,
  locale: Locale = 'en',
): Metadata {
  const t = createTranslator({
    locale,
    messages: getLocaleMessages(locale),
    namespace: 'ComponentPage',
  })
  const name = getComponentName(component)
  const englishPath = `/components/${component}`
  const path = locale === 'en' ? englishPath : `/${locale}${englishPath}`
  const title = `Devup UI - ${name}`
  const description = t('description', { name })
  return {
    title,
    description,
    alternates: {
      canonical: path,
      languages: {
        en: englishPath,
        ko: `/ko${englishPath}`,
        'x-default': englishPath,
      },
    },
    openGraph: {
      title,
      description,
      url: path,
      siteName: 'Devup UI',
      images: [`/components-og/${component}.webp`],
    },
  }
}

interface ComponentDocProps {
  component: string
  Index: ComponentType<MDXProps>
  Api: ComponentType<MDXProps>
  locale?: Locale
}

export async function ComponentDoc({
  component,
  Index,
  Api,
  locale = 'en',
}: ComponentDocProps) {
  const t = await getTranslations({ locale, namespace: 'ComponentPage' })
  const c = await getDemos(component)
  const m = Math.ceil(c.length / 2)
  const componentName = getComponentName(component)

  return (
    <VStack gap="16px" maxW="100%" overflow="hidden">
      <Text as="strong" color="$primary" m="0" typography="captionBold">
        {componentName}
      </Text>
      <Text as="h1" color="$title" m="0" typography="h4">
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
        <Text as="h2" color="$title" m="0" typography="h6">
          {t('examples')}
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
