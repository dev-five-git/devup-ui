import { VStack } from '@devup-ui/react'

import { URL_PREFIX } from '../../../constants'
import { MenuItem } from './MenuItem'

export function LeftMenu() {
  return (
    <VStack gap="6px" h="1008px" p="20px 16px" w="220px">
      <MenuItem to={URL_PREFIX + '/docs/overview'}>Overview</MenuItem>
      <MenuItem to={URL_PREFIX + '/docs/installation'}>Installation</MenuItem>
      <MenuItem to={URL_PREFIX + '/docs/features'}>Features</MenuItem>
      <MenuItem
        subMenu={[
          {
            to: URL_PREFIX + '/docs/api/box',
            children: 'Box',
          },
          {
            to: URL_PREFIX + '/docs/api/button',
            children: 'Button',
          },
          {
            to: URL_PREFIX + '/docs/api/input',
            children: 'Input',
          },
          {
            to: URL_PREFIX + '/docs/api/input',
            children: 'Text',
          },
          {
            to: URL_PREFIX + '/docs/api/image',
            children: 'Image',
          },
          {
            to: URL_PREFIX + '/docs/api/flex',
            children: 'Flex',
          },
          {
            to: URL_PREFIX + '/docs/api/v-stack',
            children: 'VStack',
          },
          {
            to: URL_PREFIX + '/docs/api/center',
            children: 'Center',
          },
          {
            to: URL_PREFIX + '/docs/api/css',
            children: 'css',
          },
          {
            to: URL_PREFIX + '/docs/api/style-props',
            children: 'Style Props',
          },
          {
            to: URL_PREFIX + '/docs/api/selector',
            children: 'Selector',
          },
          {
            to: URL_PREFIX + '/docs/api/group-selector',
            children: 'Group Selector',
          },
        ]}
      >
        API
      </MenuItem>
      <MenuItem
        subMenu={[
          {
            to: URL_PREFIX + '/docs/devup/devup-json',
            children: 'What is devup?',
          },
          {
            to: URL_PREFIX + '/docs/devup/colors',
            children: 'Colors',
          },
          {
            to: URL_PREFIX + '/docs/devup/typography',
            children: 'Typography',
          },
          {
            to: URL_PREFIX + '/docs/devup/breakpoints',
            children: 'Breakpoints',
          },
        ]}
      >
        Devup
      </MenuItem>
    </VStack>
  )
}
