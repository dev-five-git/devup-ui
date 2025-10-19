import { VStack } from '@devup-ui/react'

import { MenuItem } from './MenuItem'

export function LeftMenu() {
  return (
    <VStack gap="6px">
      <MenuItem to="/docs/overview">Overview</MenuItem>
      <MenuItem to="/docs/installation">Installation</MenuItem>
      <MenuItem
        subMenu={[
          { to: '/docs/core-concepts/zero-runtime', children: 'Zero Runtime' },
          {
            to: '/docs/core-concepts/no-dependencies',
            children: 'No Dependencies',
          },
          {
            to: '/docs/core-concepts/style-storage',
            children: 'Style Storage',
          },
          {
            to: '/docs/core-concepts/type-inference-system',
            children: 'Type Inference System',
          },
          {
            to: '/docs/core-concepts/optimize-css',
            children: 'Optimize CSS',
          },
          {
            to: '/docs/core-concepts/nm-base',
            children: 'N/M Base',
          },
        ]}
      >
        Core Concepts
      </MenuItem>
      <MenuItem to="/docs/features">Features</MenuItem>
      <MenuItem
        subMenu={[
          {
            to: '/docs/figma-and-theme-integration/devup-figma-plugin',
            children: 'Devup Figma Plugin',
          },
          {
            to: '/docs/figma-and-theme-integration/devup-json',
            children: 'devup.json Configuration',
          },
        ]}
      >
        Figma and Theme Integration
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
            children: 'Style Props',
          },
          {
            to: '/docs/api/selector',
            children: 'Selector',
          },
          {
            to: '/docs/api/group-selector',
            children: 'Group Selector',
          },
        ]}
      >
        API
      </MenuItem>
      <MenuItem
        subMenu={[
          {
            to: '/docs/devup/devup-json',
            children: 'What is devup?',
          },
          {
            to: '/docs/devup/colors',
            children: 'Colors',
          },
          {
            to: '/docs/devup/typography',
            children: 'Typography',
          },
          {
            to: '/docs/devup/breakpoints',
            children: 'Breakpoints',
          },
          {
            to: '/docs/devup/figma-plugin',
            children: 'Figma Plugin',
          },
        ]}
      >
        Devup
      </MenuItem>
    </VStack>
  )
}
