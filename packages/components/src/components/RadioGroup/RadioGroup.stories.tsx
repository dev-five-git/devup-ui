import { RadioGroup } from './index'

export default {
  title: 'Devfive/RadioGroup',
  component: RadioGroup,
}

export const Default = {
  args: {
    disabled: false,
    name: 'radio',
    colors: {
      primary: 'var(--primary)',
      border: 'var(--border)',
      text: 'var(--text)',
      bg: 'var(--bg)',
      hoverBg: 'var(--hoverBg)',
      hoverBorder: 'var(--hoverBorder)',
      hoverColor: 'var(--hoverColor)',
      checkedBg: 'var(--checkedBg)',
      checkedBorder: 'var(--checkedBorder)',
      checkedColor: 'var(--checkedColor)',
      disabledBg: 'var(--disabledBg)',
      disabledColor: 'var(--disabledColor)',
    },
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
