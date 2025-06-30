import { VStack } from '@devup-ui/react'

import { MenuItem } from '../docs/MenuItem'

export function LeftMenu() {
  return (
    <VStack gap="6px">
      <MenuItem to="/components/overview">Overview</MenuItem>
      <MenuItem
        subMenu={[
          { to: '/components/form/button', children: 'Button' },
          { to: '/components/form/text-box', children: 'Text box' },
          { to: '/components/form/text-area', children: 'Text area' },
          { to: '/components/form/dropdown', children: 'Dropdown' },
          { to: '/components/form/radio', children: 'Radio' },
          { to: '/components/form/checkbox', children: 'Checkbox' },
          { to: '/components/form/stepper', children: 'Stepper' },
          { to: '/components/form/toggle', children: 'Toggle' },
          { to: '/components/form/slider', children: 'Slider' },
          { to: '/components/form/date-picker', children: 'Date picker' },
          { to: '/components/form/color-picker', children: 'Color picker' },
          { to: '/components/form/uploader', children: 'Uploader' },
          { to: '/components/form/pagination', children: 'Pagination' },
          { to: '/components/form/progress-bar', children: 'Progress Bar' },
          { to: '/components/form/search', children: 'Search' },
          { to: '/components/form/select', children: 'Select' },
          { to: '/components/form/label', children: 'Label' },
        ]}
      >
        Form
      </MenuItem>
      <MenuItem
        subMenu={[
          { to: '/components/layout/footer', children: 'Footer' },
          { to: '/components/layout/tooltip', children: 'Tooltip' },
          { to: '/components/layout/tab', children: 'Tab' },
          { to: '/components/layout/menu', children: 'Menu' },
          { to: '/components/layout/header', children: 'Header' },
          { to: '/components/layout/confirm', children: 'Confirm' },
          { to: '/components/layout/snackbar', children: 'Snackbar' },
          { to: '/components/layout/bottom-sheet', children: 'Bottom sheet' },
        ]}
      >
        Layout
      </MenuItem>
      <MenuItem
        subMenu={[
          { to: '/components/theme/theme-button', children: 'Theme Button' },
        ]}
      >
        Theme
      </MenuItem>
    </VStack>
  )
}
