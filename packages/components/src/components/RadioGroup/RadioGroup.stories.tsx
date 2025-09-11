import { RadioGroup } from './index'

export default {
  title: 'Devfive/RadioGroup',
  component: RadioGroup,
}

export const Default = {
  args: {
    disabled: false,
    name: 'radio',
    options: [
      {
        value: '1',
        label: '옵션 1',
      },
      {
        value: '2',
        label: '옵션 2',
      },
      {
        value: '3',
        label: '옵션 3',
      },
    ],
    variant: 'default',
    direction: 'row',
  },
}
