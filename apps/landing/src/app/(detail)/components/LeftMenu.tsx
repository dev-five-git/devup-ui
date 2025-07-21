import { VStack } from '@devup-ui/react'

import { MenuItem } from '../docs/MenuItem'

export function LeftMenu() {
  return (
    <VStack gap="6px">
      <MenuItem to="/components/overview">Overview</MenuItem>
      <MenuItem
        subMenu={[
          { to: '/components/button', children: 'Button' },
          { to: '/components/textbox', children: 'Textbox' },
          { to: '/components/textarea', children: 'Textarea' },
          { to: '/components/dropdown', children: 'Dropdown' },
          { to: '/components/radio', children: 'Radio' },
          { to: '/components/checkbox', children: 'Checkbox' },
          { to: '/components/stepper', children: 'Stepper' },
          { to: '/components/toggle', children: 'Toggle' },
          { to: '/components/slider', children: 'Slider' },
          { to: '/components/date-picker', children: 'Date Picker' },
          { to: '/components/color-picker', children: 'Color Picker' },
          { to: '/components/uploader', children: 'Uploader' },
          { to: '/components/pagination', children: 'Pagination' },
          { to: '/components/progress-bar', children: 'Progress Bar' },
          { to: '/components/search', children: 'Search' },
          { to: '/components/select', children: 'Select' },
          { to: '/components/label', children: 'Label' },
        ]}
      >
        Form
      </MenuItem>
      <MenuItem
        subMenu={[
          { to: '/components/footer', children: 'Footer' },
          { to: '/components/tooltip', children: 'Tooltip' },
          { to: '/components/tab', children: 'Tab' },
          { to: '/components/menu', children: 'Menu' },
          { to: '/components/header', children: 'Header' },
          { to: '/components/confirm', children: 'Confirm' },
          { to: '/components/snackbar', children: 'Snackbar' },
          { to: '/components/bottom-sheet', children: 'Bottom Sheet' },
        ]}
      >
        Layout
      </MenuItem>
      <MenuItem
        subMenu={[{ to: '/components/theme-button', children: 'Theme Button' }]}
      >
        Theme
      </MenuItem>
    </VStack>
  )
}
