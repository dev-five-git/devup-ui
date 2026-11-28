import { Center, css, Flex, Grid, Text, VStack } from '@devup-ui/react'
import type { Locale } from 'next-intl'
import { getTranslations } from 'next-intl/server'
import type { ComponentType } from 'react'

import { Icons } from '@/components/icons/components'
import { COMPONENT_GROUPS } from '@/constants'
import { Link } from '@/i18n/navigation'

import Card from '../Card'
import { getComponentName } from '../ComponentDoc'

const GROUP_LABELS = {
  Form: 'form',
  Layout: 'layout',
  Theme: 'theme',
} as const

function ComponentCard({ component }: { component: string }) {
  const Icon =
    Icons[`Icon${getComponentName(component).replaceAll(' ', '')}Comp`]
  return (
    <Link
      className={css({ textDecoration: 'none' })}
      href={`/components/${component}`}
    >
      <Card>
        <Center h="140px">
          <Icon className={css({ w: '100%' })} />
        </Center>
        <Flex
          alignItems="center"
          borderTop="1px solid $border"
          gap="10px"
          px="16px"
          py="12px"
        >
          <Text color="$text" textAlign="right" typography="buttonSmid">
            {getComponentName(component)}
          </Text>
        </Flex>
      </Card>
    </Link>
  )
}

export async function ComponentsOverview({
  Description,
  locale = 'en',
}: {
  Description: ComponentType
  locale?: Locale
}) {
  const t = await getTranslations({ locale, namespace: 'ComponentsOverview' })
  const menu = await getTranslations({ locale, namespace: 'ComponentsMenu' })
  return (
    <VStack gap="16px">
      <Text color="$primary" typography="captionBold">
        {menu('overview')}
      </Text>
      <Text as="h1" color="$title" m="0" typography="h4">
        {t('title')}
      </Text>
      <Text color="$text" typography="bodyReg">
        <Description />
      </Text>
      <Text as="h2" color="$title" m="0" pt="30px" typography="h4">
        {t('examples')}
      </Text>
      {(Object.keys(GROUP_LABELS) as (keyof typeof GROUP_LABELS)[]).map(
        (group, index) => (
          <VStack
            key={group}
            gap="16px"
            overflow="visible"
            pb="30px"
            pt={index === 0 ? undefined : '30px'}
          >
            <Text as="h3" color="$title" m="0" typography="h6">
              {menu(GROUP_LABELS[group])}
            </Text>
            <Grid
              gap={['10px', null, null, null, '20px']}
              gridTemplateColumns={[
                'repeat(1, 1fr)',
                'repeat(3, 1fr)',
                null,
                'repeat(4, 1fr)',
                'repeat(5, 1fr)',
              ]}
              overflow="visible"
            >
              {COMPONENT_GROUPS[group].map((component) => (
                <ComponentCard key={component} component={component} />
              ))}
            </Grid>
          </VStack>
        ),
      )}
    </VStack>
  )
}
