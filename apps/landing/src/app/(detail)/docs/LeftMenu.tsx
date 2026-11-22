'use client'
import { VStack } from '@devup-ui/react'
import { useTranslations } from 'next-intl'

import { MenuItem } from './MenuItem'

export function LeftMenu() {
  const t = useTranslations('DocsMenu')
  return (
    <VStack gap="6px">
      <MenuItem to="/docs/overview">{t('overview')}</MenuItem>
      <MenuItem to="/docs/quick-start">{t('quickStart')}</MenuItem>
      <MenuItem to="/docs/installation">{t('installation')}</MenuItem>
      <MenuItem
        subMenu={[
          { to: '/docs/core-concepts/zero-runtime', children: t('zeroRuntime') },
          {
            to: '/docs/core-concepts/no-dependencies',
            children: t('noDependencies'),
          },
          {
            to: '/docs/core-concepts/style-storage',
            children: t('styleStorage'),
          },
          {
            to: '/docs/core-concepts/type-inference-system',
            children: t('typeInferenceSystem'),
          },
          {
            to: '/docs/core-concepts/optimize-css',
            children: t('optimizeCss'),
          },
          {
            to: '/docs/core-concepts/nm-base',
            children: t('nmBase'),
          },
        ]}
      >
        {t('coreConcepts')}
      </MenuItem>
      <MenuItem to="/docs/features">{t('features')}</MenuItem>
      <MenuItem to="/docs/limitations">{t('limitations')}</MenuItem>
      <MenuItem to="/docs/build-errors">{t('buildErrors')}</MenuItem>
      <MenuItem
        subMenu={[
          {
            to: '/docs/migration/overview',
            children: t('migrationOverview'),
          },
          {
            to: '/docs/migration/styled-components',
            children: 'styled-components & Emotion',
          },
          {
            to: '/docs/migration/vanilla-extract',
            children: 'vanilla-extract',
          },
          {
            to: '/docs/migration/stylex',
            children: 'StyleX',
          },
          {
            to: '/docs/migration/tailwind',
            children: 'Tailwind CSS',
          },
        ]}
      >
        {t('migration')}
      </MenuItem>
      <MenuItem
        subMenu={[
          {
            to: '/docs/figma-and-theme-integration/devup-figma-plugin',
            children: t('devupFigmaPlugin'),
          },
          {
            to: '/docs/figma-and-theme-integration/devup-json',
            children: t('devupJsonConfiguration'),
          },
        ]}
      >
        {t('figmaAndThemeIntegration')}
      </MenuItem>
      <MenuItem
        subMenu={[
          {
            to: '/docs/api/box',
            children: 'Box',
          },
          {
            to: '/docs/api/button',
            children: 'Button',
          },
          {
            to: '/docs/api/input',
            children: 'Input',
          },
          {
            to: '/docs/api/text',
            children: 'Text',
          },
          {
            to: '/docs/api/image',
            children: 'Image',
          },
          {
            to: '/docs/api/flex',
            children: 'Flex',
          },
          {
            to: '/docs/api/v-stack',
            children: 'VStack',
          },
          {
            to: '/docs/api/center',
            children: 'Center',
          },
          {
            to: '/docs/api/grid',
            children: 'Grid',
          },
          {
            to: '/docs/api/css',
            children: 'css',
          },
          {
            to: '/docs/api/style-props',
            children: t('styleProps'),
          },
          {
            to: '/docs/api/custom-shorthands',
            children: t('customShorthands'),
          },
          {
            to: '/docs/api/selector',
            children: t('selector'),
          },
          {
            to: '/docs/api/group-selector',
            children: t('groupSelector'),
          },
        ]}
      >
        API
      </MenuItem>
      <MenuItem
        subMenu={[
          {
            to: '/docs/devup/devup-json',
            children: t('whatIsDevup'),
          },
          {
            to: '/docs/devup/colors',
            children: t('colors'),
          },
          {
            to: '/docs/devup/typography',
            children: t('typography'),
          },
          {
            to: '/docs/devup/length',
            children: t('length'),
          },
          {
            to: '/docs/devup/shadow',
            children: t('shadow'),
          },
          {
            to: '/docs/devup/breakpoints',
            children: t('breakpoints'),
          },
          {
            to: '/docs/devup/figma-plugin',
            children: t('figmaPlugin'),
          },
        ]}
      >
        Devup
      </MenuItem>
    </VStack>
  )
}
