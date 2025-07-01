import { Button } from './index'

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
export default {
  title: 'Devfive/Button',
  component: Button,
  tags: ['autodocs'],
}

export const Default = {
  args: {
    children: 'Button Text',
    variant: 'default',
    disabled: false,
  },
}
